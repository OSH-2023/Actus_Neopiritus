mod test_common;

extern crate driver_block;
extern crate axtask;
extern crate axfs;
use driver_block::ramdisk::RamDisk;

const IMG_PATH: &str = "resources/fat16.img";

fn make_disk() -> std::io::Result<RamDisk> {
    let path = std::env::current_dir()?.join(IMG_PATH);
    println!("Loading disk image from {:?} ...", path);
    let data = std::fs::read(path)?;
    println!("size = {} bytes", data.len());
    Ok(RamDisk::from(&data))
}

fn main() {
    println!("Testing fatfs with ramdisk ...");

    let disk = make_disk().expect("failed to load disk image");
    axtask::init_scheduler(); // call this to use `axsync::Mutex`.
    axfs::init_filesystems(disk);   //这个地方VSCODE插件可能会误报，不用管他！

    test_common::test_all();
}
