use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use spin::RwLock;

use crate::printk;

use super::vfs::{split_path, SnDirEntry, SnVfsError, SnVfsFilesystem, SnVfsNode, SnVfsNodeRef, SnVfsResult, SnVfsType};

struct SnDummyNode {
    pub name: &'static str,
    pub node_type: SnVfsType,
    contents: Option<&'static [u8]>,
    children: Option<RwLock<BTreeMap<&'static str, SnVfsNodeRef>>>,
}
impl SnVfsNode for SnDummyNode {
    fn is_file(&self) -> bool {
        match self.node_type {
            SnVfsType::File => true,
            _ => false,
        }
    }

    fn is_dir(&self) -> bool {
        match self.node_type {
            SnVfsType::Dir => true,
            _ => false,
        }
    }

    fn read(&self, buf: &mut [u8]) -> Result<usize, SnVfsError> {
        if let Some(content) = &self.contents {
            let file = content;
            buf[0..file.len()].copy_from_slice(&file);

            return Ok(file.len());
        }

        Err(SnVfsError::ReadError)
    }

    fn len(&self) -> usize {
        if let Some(content) = &self.contents {
            let file = content;
            return file.len();
        }

        0
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn read_dir(&self, entries: &mut [super::vfs::SnDirEntry]) {
        let mut ent = Vec::<SnDirEntry>::new();
        if let Some(dir) = &self.children {
            let dirs = dir.read();

            for (_, entry) in dirs.iter()  {
                match &entry.node_type() {
                    SnVfsType::File => ent.push(SnDirEntry {
                        name: entry.name(),
                        dir_type: SnVfsType::File,
                    }),
                    SnVfsType::Dir => ent.push(SnDirEntry {
                        name: entry.name(),
                        dir_type: SnVfsType::Dir,
                    }),
                }
            }
        }
        let vec = ent.as_slice();
        entries[0..vec.len()].copy_from_slice(&vec);
    }
    
    fn node_type(&self) -> SnVfsType {
        self.node_type
    }
    
    fn find(self: Arc<Self>, path: &str) -> SnVfsResult {
        let (name, sub) = split_path(path);

        let node = match name {
            "" | "." => Ok(self as SnVfsNodeRef),
            _ => self.children.as_ref().unwrap().read().get(name).cloned().ok_or(SnVfsError::NotFound),
        }?;
        
        if let Some(sub)= sub {
            node.find(sub)
        } else {
            Ok(node)
        }
    }
}
pub struct SnDummyFilesystem {
    root: SnVfsNodeRef,
}

impl SnVfsFilesystem for SnDummyFilesystem {
    fn startup(&self) {}
    
    fn root(&self) -> SnVfsNodeRef {
        return self.root.clone();
    }
}

pub fn new_example_filesystem() -> SnDummyFilesystem {
    printk!("fs::dummy: creating a sample rootfs");
    SnDummyFilesystem {
        root: Arc::new(
            SnDummyNode {
                node_type: SnVfsType::Dir,
                name: "",
                contents: None,
                children: Some(RwLock::new(
                    BTreeMap::from([
                        ("shinosawa", Arc::new(SnDummyNode{
                            name: "shinosawa",
                            node_type: SnVfsType::Dir,
                            contents: None,
                            children: Some(RwLock::new(
                                BTreeMap::from([
                                    ("system", Arc::new(SnDummyNode{
                                        name: "system",
                                        node_type: SnVfsType::Dir,
                                        contents: None,
    
                                        children: Some(RwLock::new(
                                            BTreeMap::from([
                                                ("kotono", Arc::new(SnDummyNode {
                                                    name: "kotono",
                                                    node_type: SnVfsType::File,
                                                    contents: Some(include_bytes!("../../../../../target/x86_64-shinosawa/release/kotono")),
                                                    children: None,
                                                }) as SnVfsNodeRef)
                                            ])
                                        )),
                                    }) as SnVfsNodeRef)
                                ]),
                            )),
                        }) as SnVfsNodeRef)
                    ])
                )),
            }
        ) as SnVfsNodeRef,
    }
}
