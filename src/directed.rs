//! A directed two-phase channel.
//! The channel provides two data pointers, one of which is read-only.
//! Data is only transmitted from the writable end to the readable end.

use crate::{ChannelKey, DataKey};

/// A directed channel used for communication between threads.
/// It holds two instances of `Data`, which can be accessed or flushed.
/// One `Data` can only be read, and the other can only be written to.
/// A flush copies the writable data into the read-only data.
///
/// At any time, either references to `Data` can exist, or a flush operation can be performed.
/// This allows to different threads to hold pointers to one of the `Data` fields each,
/// and a third thread to flush the content of these `Data` fields, resulting in directed inter-thread communication.
///
/// See [DirectedChannel::create] for more info.
#[derive(Debug)]
pub struct DirectedChannel<Data> {
    pub(crate) read_only: Data,
    pub(crate) writable: Data,
}

/// A pointer to a directed channel.
/// It can only be accessed using a [ChannelKey].
///
/// This type should always be destroyed via the [DirectedChannel::destroy] or [DirectedChannelPointer::destroy] method to ensure soundness (at runtime).
#[derive(Debug)]
#[must_use]
pub struct DirectedChannelPointer<Data> {
    channel: Box<DirectedChannel<Data>>,
}

/// A pointer to the read-only data field in a directed channel.
/// It can only be accessed using a [DataKey].
///
/// This type should always be destroyed via the [DirectedChannel::destroy] or [DirectedChannelPointer::destroy] method to ensure soundness (at runtime).
#[derive(Debug)]
#[must_use]
pub struct ReadOnlyDataPointer<Data> {
    pub(crate) data: *const Data,
}

/// A pointer to the writable data field in a directed channel.
/// It can only be accessed using a [DataKey].
///
/// This type should always be destroyed via the [DirectedChannel::destroy] or [DirectedChannelPointer::destroy] method to ensure soundness (at runtime).
#[derive(Debug)]
#[must_use]
pub struct WritableDataPointer<Data> {
    pub(crate) data: *mut Data,
}

impl<Data> DirectedChannel<Data> {
    /// Create a directed channel and hand out three pointers to it.
    /// One [DirectedChannelPointer] used to flush (copy) the content of the writable `Data` field into the read-only data field,
    /// one [ReadOnlyDataPointer] used to read from the directed channel, and
    /// one [WritableDataPointer] used to write to the directed channel.
    ///
    /// Note that the `ReadOnlyPointer` and the `WriteOnlyPointer` point to different copies of `Data`,
    /// and hence can safely be accessed concurrently.
    /// See [`DirectedChannelPointer::flush`] for how to exchange information between the pointers.
    pub fn create(
        read_only: Data,
        writable: Data,
    ) -> (
        DirectedChannelPointer<Data>,
        ReadOnlyDataPointer<Data>,
        WritableDataPointer<Data>,
    ) {
        let mut channel_pointer = DirectedChannelPointer {
            channel: Box::new(DirectedChannel {
                read_only,
                writable,
            }),
        };
        let read_only_data_pointer = ReadOnlyDataPointer {
            data: (&channel_pointer.channel.read_only) as *const Data,
        };
        let writable_data_pointer = WritableDataPointer {
            data: (&mut channel_pointer.channel.writable) as *mut Data,
        };
        (
            channel_pointer,
            read_only_data_pointer,
            writable_data_pointer,
        )
    }

    /// Destroys the directed channel linked with the given pointers (see [DirectedChannel::create]).
    /// Compared to [`DirectedChannel::destroy_single`], this function accepts multiple [`ReadOnlyDataPointer`]s for destruction.
    ///
    /// **Panics** if not all pointers point to the same channel.
    pub fn destroy(
        channel_pointer: DirectedChannelPointer<Data>,
        read_only_data_pointers: impl IntoIterator<Item = ReadOnlyDataPointer<Data>>,
        writable_data_pointer: WritableDataPointer<Data>,
    ) -> (Data, Data) {
        let DirectedChannelPointer { mut channel } = channel_pointer;
        let channel_writable_data_pointer = (&mut channel.writable) as *mut Data;
        let WritableDataPointer {
            data: writable_data_pointer,
        } = writable_data_pointer;
        assert_eq!(channel_writable_data_pointer, writable_data_pointer);
        let channel_read_only_data_pointer = (&channel.read_only) as *const Data;

        for read_only_data_pointer in read_only_data_pointers {
            let ReadOnlyDataPointer {
                data: read_only_data_pointer,
            } = read_only_data_pointer;
            assert_eq!(channel_read_only_data_pointer, read_only_data_pointer);
        }

        (channel.read_only, channel.writable)
    }

    /// Destroys the directed channel linked with the given pointers (see [DirectedChannel::create]).
    /// Compared to [`DirectedChannel::destroy`], this function accepts only one [`ReadOnlyDataPointer`] for destruction.
    ///
    /// **Panics** if not all pointers point to the same channel.
    pub fn destroy_single(
        channel_pointer: DirectedChannelPointer<Data>,
        read_only_data_pointer: ReadOnlyDataPointer<Data>,
        writable_data_pointer: WritableDataPointer<Data>,
    ) -> (Data, Data) {
        Self::destroy(
            channel_pointer,
            [read_only_data_pointer],
            writable_data_pointer,
        )
    }
}

impl<Data: Clone> DirectedChannel<Data> {
    /// In this constructor, both `Data` fields are initialised equally from the given `Data`.
    ///
    /// See [`DirectedChannel::create`] for more details.
    pub fn create_equal(
        data: Data,
    ) -> (
        DirectedChannelPointer<Data>,
        ReadOnlyDataPointer<Data>,
        WritableDataPointer<Data>,
    ) {
        Self::create(data.clone(), data)
    }

    pub fn flush(&mut self, #[allow(unused)] channel_key: &ChannelKey) {
        self.read_only = self.writable.clone();
    }
}

impl<Data: Clone> DirectedChannelPointer<Data> {
    /// Clone the writable `Data` into the read-only `Data`.
    pub fn flush(&mut self, #[allow(unused)] channel_key: &ChannelKey) {
        let channel: &mut DirectedChannel<Data> = &mut self.channel;
        channel.read_only = channel.writable.clone();
    }
}

impl<Data> DirectedChannelPointer<Data> {
    /// Shorthand for [DirectedChannel::destroy].
    pub fn destroy(
        self,
        read_only_data_pointers: impl IntoIterator<Item = ReadOnlyDataPointer<Data>>,
        writable_data_pointer: WritableDataPointer<Data>,
    ) -> (Data, Data) {
        DirectedChannel::destroy(self, read_only_data_pointers, writable_data_pointer)
    }

    /// Shorthand for [DirectedChannel::destroy_single].
    pub fn destroy_single(
        self,
        read_only_data_pointer: ReadOnlyDataPointer<Data>,
        writable_data_pointer: WritableDataPointer<Data>,
    ) -> (Data, Data) {
        DirectedChannel::destroy_single(self, read_only_data_pointer, writable_data_pointer)
    }
}

impl<Data> ReadOnlyDataPointer<Data> {
    /// Get a reference to the `Data` field pointed to by this pointer.
    pub fn get(&self, #[allow(unused)] data_key: &DataKey) -> &Data {
        unsafe { &*self.data }
    }
}

impl<Data> WritableDataPointer<Data> {
    /// Get a reference to the `Data` field pointed to by this pointer.
    pub fn get(&self, #[allow(unused)] data_key: &DataKey) -> &Data {
        unsafe { &*self.data }
    }

    /// Get a mutable reference to the `Data` field pointed to by this pointer.
    pub fn get_mut(&mut self, #[allow(unused)] data_key: &DataKey) -> &mut Data {
        unsafe { &mut *self.data }
    }
}

impl<Data> Clone for ReadOnlyDataPointer<Data> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Data> Copy for ReadOnlyDataPointer<Data> {}

unsafe impl<Data> Send for DirectedChannelPointer<Data> {}
unsafe impl<Data> Send for ReadOnlyDataPointer<Data> {}
unsafe impl<Data> Send for WritableDataPointer<Data> {}

unsafe impl<Data> Sync for DirectedChannelPointer<Data> {}
unsafe impl<Data> Sync for ReadOnlyDataPointer<Data> {}
unsafe impl<Data> Sync for WritableDataPointer<Data> {}

/// Object-safe trait for [`DirectedChannelPointer`]s.
pub trait IDirectedChannel: Send + Sync {
    /// Perform the [`DirectedChannelPointer::flush`] operation.
    fn flush(&mut self, channel_key: &ChannelKey);
}

impl<Data: Clone> IDirectedChannel for DirectedChannelPointer<Data> {
    fn flush(&mut self, channel_key: &ChannelKey) {
        DirectedChannelPointer::flush(self, channel_key);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        directed::{DirectedChannel, IDirectedChannel},
        MasterKey,
    };

    #[test]
    fn test() {
        let mut master_key = unsafe { MasterKey::create_unlimited() };
        let (mut channel_pointer, read_only_data_pointer, mut writable_data_pointer) =
            DirectedChannel::create(0, 0);

        for i in 0..3 {
            let data_key = master_key.get_data_key();
            assert_eq!(*read_only_data_pointer.get(&data_key), i);
            *writable_data_pointer.get_mut(&data_key) = i + 1;

            let channel_key = data_key.into_channel_key();
            channel_pointer.flush(&channel_key);
        }

        let (read_only_data, writable_data) = DirectedChannel::destroy_single(
            channel_pointer,
            read_only_data_pointer,
            writable_data_pointer,
        );
        assert_eq!(read_only_data, 3);
        assert_eq!(writable_data, 3);
    }

    #[test]
    fn ensure_channel_is_object_safe() {
        let mut master_key = unsafe { MasterKey::create_unlimited() };
        let (mut channel, read_only_data_pointer, writable_data_pointer) =
            DirectedChannel::create(1, 2);
        let dyn_channel: &mut dyn IDirectedChannel = &mut channel;

        dyn_channel.flush(&master_key.get_channel_key());
        assert_eq!(*read_only_data_pointer.get(&master_key.get_data_key()), 2);
        assert_eq!(*writable_data_pointer.get(&master_key.get_data_key()), 2);
        DirectedChannel::destroy_single(channel, read_only_data_pointer, writable_data_pointer);
    }
}
