use crate::dev::Disk;
use alloc::sync::Arc;
use axfs_vfs::VfsOps;

/*
以下实现Ext文件系统！！！！！！！！！！！
其实就是从Github仓库https://github.com/pi-pi3/ext2-rs.git
克隆下来，魔改一番再套层壳，测试一下就可以了。
*/

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
