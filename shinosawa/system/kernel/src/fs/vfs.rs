use alloc::{collections::BTreeMap, sync::Arc};
use conquer_once::spin::OnceCell;
use spin::RwLock;

use crate::printk;

pub type SnVfsNodeRef = Arc<dyn SnVfsNode>;

pub type SnVfsResult = Result<super::vfs::SnVfsNodeRef, SnVfsError> ;

#[derive(Debug)]
pub enum SnVfsError {
    ReadError,
    NotFound,
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

    fn node_type(&self) -> SnVfsType;

    fn read(&self, buf: &mut [u8]) -> Result<usize, SnVfsError>;
    fn read_dir(&self, entries: &mut [SnDirEntry]);

    fn find(self: Arc<Self>, path: &str) -> SnVfsResult;
}

pub trait SnVfsFilesystem: Send + Sync {
    fn startup(&self);

    fn root(&self) -> SnVfsNodeRef;
}

pub struct SnVfs {
    filesystem: BTreeMap<&'static str, Arc<dyn SnVfsFilesystem>>,
}

impl SnVfs {
    pub fn attach(&mut self, drive: &'static str, fs: impl SnVfsFilesystem + 'static) {
        let drive = drive;
        self.filesystem.insert(drive, Arc::new(fs));
    }
}
pub static VFS: OnceCell<RwLock<SnVfs>> = OnceCell::uninit();

pub fn init() {
    printk!("fs::vfs: initializing VFS interface");
    VFS.init_once(move || {
        RwLock::new(SnVfs {
            filesystem: BTreeMap::new(),
        })
    });
}

pub fn attach(drive: &'static str, fs: impl SnVfsFilesystem + 'static) {
    printk!("fs::vfs: attaching a filesystem");
    let mut vfs = VFS.get().unwrap().write();

    vfs.attach(drive, fs);
}

/// Splits the first path name and the rest of the path
pub fn split_path(path: &str) -> (&str, Option<&str>) {
    let path = path.trim_start_matches("/");

    path.find("/").map_or((path, None), |f| {
        (&path[..f], Some(&path[f+1..]))
    })
}

pub fn find(path: &str) -> Result<SnVfsNodeRef, SnVfsError> {
    let mut vfs = VFS.get().unwrap().read();
    let (name, sub) = split_path(path);

    let node= vfs.filesystem.get(name).cloned().ok_or(SnVfsError::NotFound)?;
    
    if let Some(sub)= sub {
        node.root().find(sub)
    } else {
        Err(SnVfsError::NotFound)
    }
}