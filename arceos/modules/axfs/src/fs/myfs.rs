use crate::dev::Disk;
use alloc::sync::Arc;
use axfs_vfs::VfsOps;

/*
以下实现Ext文件系统！！！！！！！！！！！
其实就是从Github仓库https://github.com/pi-pi3/ext2-rs.git
克隆下来，魔改一番再套层壳，测试一下就可以了。
*/
use ext2::fs::{self, sync};
use ext2::sector::Size512 as Sector;
use ext2::volume::Volume;

impl Volume<u8, Sector> for Disk {
    type Error = ext2::error::Error;
    //TODO：实现四个函数。
    fn commit(
            &mut self,
            slice: Option<ext2::volume::VolumeCommit<u8, Sector>>,
        ) -> Result<(), Self::Error> {
        unimplemented!()
    }
    fn size(&self) -> ext2::volume::size::Size<Sector> {
        unimplemented!()
    }
    fn slice<'a>(
            &'a self,
            range: core::ops::Range<ext2::sector::Address<Sector>>,
        ) -> Result<ext2::volume::VolumeSlice<'a, u8, Sector>, Self::Error> {
        unimplemented!()
    }
    unsafe fn slice_unchecked<'a>(
            &'a self,
            range: core::ops::Range<ext2::sector::Address<Sector>>,
        ) -> ext2::volume::VolumeSlice<'a, u8, Sector> {
        unimplemented!()
    }
}

pub struct Ext2FileSystem {
    inner : sync::Synced<fs::Ext2<Sector, Disk>>
    // MAYBE something else
}

impl VfsOps for Ext2FileSystem {
    //TODO 实现这个函数
    fn root_dir(&self) -> axfs_vfs::VfsNodeRef {
        unimplemented!()
    }
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
