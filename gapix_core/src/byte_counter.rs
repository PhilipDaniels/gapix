use std::io::{self, Seek, Write};

pub(crate) struct ByteCounter<W> {
    inner: W,
    count: usize,
}

impl<W> ByteCounter<W>
where
    W: Write,
{
    pub(crate) fn new(inner: W) -> Self {
        ByteCounter { inner, count: 0 }
    }

    // fn into_inner(self) -> W {
    //     self.inner
    // }

    pub(crate) fn bytes_written(&self) -> usize {
        self.count
    }
}

impl<W> Write for ByteCounter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let res = self.inner.write(buf);
        if let Ok(size) = res {
            self.count += size
        }
        res
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W> Seek for ByteCounter<W>
where
    W: Seek,
{
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}
