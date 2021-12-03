// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

use std::os::raw::c_char;
use std::{
    ffi::{c_void, CStr},
    mem,
    pin::Pin,
};

use crate::{
    proxy::{Listener, Proxy, ProxyT},
    types::ObjectType,
};

#[derive(Debug)]
pub struct Metadata {
    proxy: Proxy,
}

impl ProxyT for Metadata {
    fn type_() -> ObjectType {
        ObjectType::Metadata
    }

    fn upcast(self) -> Proxy {
        self.proxy
    }

    fn upcast_ref(&self) -> &Proxy {
        &self.proxy
    }

    unsafe fn from_proxy_unchecked(proxy: Proxy) -> Self
    where
        Self: Sized,
    {
        Self { proxy }
    }
}

impl Metadata {
    pub fn add_listener_local(&self) -> MetadataListenerLocalBuilder {
        MetadataListenerLocalBuilder {
            metadata: self,
            cbs: ListenerLocalCallbacks::default(),
        }
    }
}

pub struct MetadataListener {
    // Need to stay allocated while the listener is registered
    #[allow(dead_code)]
    events: Pin<Box<pw_sys::pw_metadata_events>>,
    listener: Pin<Box<spa_sys::spa_hook>>,
    #[allow(dead_code)]
    data: Box<ListenerLocalCallbacks>,
}

impl<'meta> Listener for MetadataListener {}

impl<'meta> Drop for MetadataListener {
    fn drop(&mut self) {
        spa::hook::remove(*self.listener);
    }
}

#[derive(Default)]
struct ListenerLocalCallbacks {
    #[allow(clippy::type_complexity)]
    property: Option<Box<dyn Fn(u32, &str, Option<&str>, &str) -> i32>>,
}

#[must_use]
pub struct MetadataListenerLocalBuilder<'meta> {
    metadata: &'meta Metadata,
    cbs: ListenerLocalCallbacks,
}

impl<'meta> MetadataListenerLocalBuilder<'meta> {
    pub fn property<F>(mut self, property: F) -> Self
    where
        F: Fn(u32, &str, Option<&str>, &str) -> i32 + 'static,
    {
        self.cbs.property = Some(Box::new(property));
        self
    }

    #[must_use]
    pub fn register(self) -> MetadataListener {
        unsafe extern "C" fn metadata_events_property(
            data: *mut c_void,
            subject: u32,
            key: *const c_char,
            type_: *const c_char,
            value: *const c_char,
        ) -> i32 {
            let callbacks = (data as *mut ListenerLocalCallbacks).as_ref().unwrap();
            let key = CStr::from_ptr(key).to_string_lossy();
            let type_ = if !type_.is_null() {
                Some(CStr::from_ptr(type_).to_string_lossy())
            } else {
                None
            };
            let value = CStr::from_ptr(value).to_string_lossy();
            callbacks.property.as_ref().unwrap()(subject, &key, type_.as_deref(), &value)
        }

        let e = unsafe {
            let mut e: Pin<Box<pw_sys::pw_metadata_events>> = Box::pin(mem::zeroed());
            e.version = pw_sys::PW_VERSION_METADATA_EVENTS;

            if self.cbs.property.is_some() {
                e.property = Some(metadata_events_property);
            }

            e
        };

        let (listener, data) = unsafe {
            let metadata = &self.metadata.proxy.as_ptr();

            let data = Box::into_raw(Box::new(self.cbs));
            let mut listener: Pin<Box<spa_sys::spa_hook>> = Box::pin(mem::zeroed());
            let listener_ptr: *mut spa_sys::spa_hook = listener.as_mut().get_unchecked_mut();
            let funcs: *const pw_sys::pw_metadata_events = e.as_ref().get_ref();

            pw_sys::pw_proxy_add_object_listener(
                metadata.cast(),
                listener_ptr.cast(),
                funcs.cast(),
                data as *mut _,
            );

            (listener, Box::from_raw(data))
        };

        MetadataListener {
            events: e,
            listener,
            data,
        }
    }
}
