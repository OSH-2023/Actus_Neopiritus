//use core::marker::PhantomData;
use alloc::sync::Arc;
use crate::dev::Disk;
use axfs_vfs::VfsOps;
use alloc::vec::Vec;
use alloc::string::String;
use axsync::Mutex;

/*
以下实现Ext文件系统！！！！！！！！！！！
其实就是从Github仓库https://github.com/pi-pi3/ext2-rs.git
克隆下来，魔改一番再套层壳，测试一下就可以了。
*/
use ext2::fs::{self, sync};
use ext2::sector::Size1024 as Sector;
use ext2::volume::{Volume, VolumeSlice};

impl Volume<u8, Sector> for Disk {
    //OK!
    type Error = ext2::error::Error;
    fn commit(
            &mut self,
            slice: Option<ext2::volume::VolumeCommit<u8, Sector>>,
        ) -> Result<(), Self::Error> {
        //unimplemented!()
        slice.map(|slice| {
            let mut buf = slice.as_ref();
            let index = slice.address().into_index();
            self.set_position(index);
            loop {
                    match self.write_one(buf) {
                    Ok(x) => {
                        if x == buf.len() {
                            break Ok(());
                        } else {
                            buf = &buf[x..];
                        }
                    }
                    Err(_) => {
                        break Err(Self::Error::Other(String::from("Commit Error")));
                    }
                }
            }
        });
        Ok(())
    }
    fn size(&self) -> ext2::volume::size::Size<Sector> {
        ext2::volume::size::Size::Bounded(
            ext2::sector::Address::from(self.size())
        )
    }
    fn slice<'a>(
            &'a self,
            range: core::ops::Range<ext2::sector::Address<Sector>>,
        ) -> Result<ext2::volume::VolumeSlice<'a, u8, Sector>, Self::Error> {
        if self.size() >= range.end.into_index() {
            unsafe { Ok(self.slice_unchecked(range)) }
        } else {
            Err(Self::Error::AddressOutOfBounds {
                sector: range.end.sector(),
                offset: range.end.offset(),
                size: range.end.sector_size()
            })
        }
    }
    unsafe fn slice_unchecked<'a>(
            &'a self,
            range: core::ops::Range<ext2::sector::Address<Sector>>,
        ) -> VolumeSlice<'a, u8, Sector> {
        let mut v : Vec<u8> = Vec::new();
        let buf : &mut [u8] = &mut [];
        let index = range.start;
        let mut len = (range.end - range.start).into_index() + 1;
        self.set_position(index.into_index());  //这里暂时写报错，必须要通过改库才能不报错
        loop {
            match self.read_one(buf) {
                Ok(x) => {
                    if x >= (len as usize) {
                        v.extend_from_slice(&buf[..(len as usize)]);
                        break;                        
                    } else {
                        v.extend_from_slice(buf);
                        len -= x as u64;
                    }
                }
                Err(_) => panic!("Something error when slice.")
            }
        }
        VolumeSlice::new_owned(
            v,
            index
        )
    }
}

pub struct Ext2FileSystem (
    sync::Synced<fs::Ext2<Sector, Disk>>
);

pub struct FileWrapper (
    Mutex<sync::Inode<Sector, Disk>>
);
unsafe impl Send for FileWrapper {}
unsafe impl Sync for FileWrapper {}

impl VfsOps for Ext2FileSystem {
    //TODO 实现这个函数
    fn root_dir(&self) -> axfs_vfs::VfsNodeRef {
        return Arc::new(FileWrapper(Mutex::new(self.0.root_inode())));
    }
}

impl axfs_vfs::VfsNodeOps for FileWrapper {

}
/*
实现结束！！！！
*/

/// The interface to define custom filesystems in user apps.
#[crate_interface::def_interface]
pub trait MyFileSystemIf {
    /// Creates a new instance of the filesystem with initialization.
    ///
    /// TODO: use generic disk type
    fn new_myfs(disk: Disk) -> Arc<dyn VfsOps>;
}

pub(crate) fn new_myfs(disk: Disk) -> Arc<dyn VfsOps> {
    crate_interface::call_interface!(MyFileSystemIf::new_myfs(disk))
}
