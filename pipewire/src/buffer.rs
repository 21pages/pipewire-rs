use super::stream::Stream;

use spa::data::Data;
use std::convert::TryFrom;
use std::ptr::NonNull;

pub struct Buffer<'s> {
    buf: NonNull<pw_sys::pw_buffer>,

    /// In Pipewire, buffers are owned by the stream that generated them.
    /// This reference ensures that this rule is respected.
    stream: &'s Stream,
}

impl Buffer<'_> {
    pub(crate) unsafe fn from_raw(
        buf: *mut pw_sys::pw_buffer,
        stream: &Stream,
    ) -> Option<Buffer<'_>> {
        NonNull::new(buf).map(|buf| Buffer { buf, stream })
    }

    pub fn datas_mut(&mut self) -> &mut [Data] {
        let buffer: *mut spa_sys::spa_buffer = unsafe { self.buf.as_ref().buffer };

        let slice_of_data = if !buffer.is_null()
            && unsafe { (*buffer).n_datas > 0 && !(*buffer).datas.is_null() }
        {
            unsafe {
                let datas = (*buffer).datas as *mut Data;
                std::slice::from_raw_parts_mut(datas, usize::try_from((*buffer).n_datas).unwrap())
            }
        } else {
            &mut []
        };

        slice_of_data
    }
}

impl Drop for Buffer<'_> {
    fn drop(&mut self) {
        unsafe {
            self.stream.queue_raw_buffer(self.buf.as_ptr());
        }
    }
}
