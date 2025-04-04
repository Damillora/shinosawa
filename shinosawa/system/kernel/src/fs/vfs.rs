use alloc::{boxed::Box, sync::Arc, vec::Vec};
use conquer_once::spin::OnceCell;
use spin::RwLock;

use crate::printk;

pub type SnVfsNodeRef = Arc<dyn SnVfsNode>;

pub enum SnVfsError {
    ReadError,
}

#[derive(Clone)]
pub enum SnVfsType {
    File,
    Dir
}

#[derive(Clone)]
pub struct SnDirEntry {
    pub name: &'static str,
    pub dir_type: SnVfsType,
}

impl Copy for SnVfsType
{

}

impl Copy for SnDirEntry
{
    
}

pub trait SnVfsNode: Send + Sync {
    fn name(&self) -> &'static str;
    fn is_file(&self) -> bool;
    fn is_dir(&self) -> bool;
    fn len(&self) -> usize;

    fn read(&self, buf: &mut [u8]) -> Result<usize, SnVfsError>;
    fn read_dir(&self, entries: &mut [SnDirEntry]);
}

pub trait SnVfsFilesystem: Send + Sync {
    fn startup(&self);
}

pub struct SnVfs {
    filesystem: Vec<Arc<dyn SnVfsFilesystem>>,
}

impl SnVfs {
    pub fn attach(&mut self, fs: impl SnVfsFilesystem + 'static) {
        self.filesystem.push(Arc::new(fs));
    }
}
pub static VFS: OnceCell<RwLock<SnVfs>> = OnceCell::uninit();

pub fn init() {
    printk!("fs::vfs: initializing VFS interface");
    VFS.init_once(move || {
        RwLock::new(SnVfs {
            filesystem: Vec::new(),
        })
    });
}

pub fn attach(fs: impl SnVfsFilesystem + 'static) {
    printk!("fs::vfs: attaching a filesystem");
    let mut vfs = VFS.get().unwrap().write();

    vfs.attach(fs);
}
