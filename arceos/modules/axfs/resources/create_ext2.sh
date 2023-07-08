cd ../../
make disk_img
cp ext2fs_fuse/target/fs.img modules/axfs/resources
mv modules/axfs/resources/fs.img modules/axfs/resources/ext2.img
cd modules/axfs/