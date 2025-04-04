use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};
use spin::RwLock;

use crate::printk;

use super::vfs::{SnDirEntry, SnVfsError, SnVfsFilesystem, SnVfsNode, SnVfsType};

struct SnDummyNode {
    pub name: &'static str,
    pub node_type: SnVfsType,
    contents: Option<RwLock<Box<[u8]>>>,
    children: Option<RwLock<BTreeMap<&'static str, SnDummyNode>>>,
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
            let file = content.read();
            buf[0..file.len()].copy_from_slice(&file);

            return Ok(file.len());
        }

        Err(SnVfsError::ReadError)
    }

    fn len(&self) -> usize {
        if let Some(content) = &self.contents {
            let file = content.read();
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

            for (name, entry) in dirs.iter()  {
                match &entry.node_type {
                    SnVfsType::File => ent.push(SnDirEntry {
                        name: entry.name,
                        dir_type: SnVfsType::File,
                    }),
                    SnVfsType::Dir => ent.push(SnDirEntry {
                        name: entry.name,
                        dir_type: SnVfsType::Dir,
                    }),
                }
            }
        }
        let vec = ent.as_slice();
        entries[0..vec.len()].copy_from_slice(&vec);
    }
}
pub struct SnDummyFilesystem {
    root: SnDummyNode,
}

impl SnVfsFilesystem for SnDummyFilesystem {
    fn startup(&self) {}
}

pub fn new_example_filesystem() -> SnDummyFilesystem {
    printk!("fs::dummy: creating a sample rootfs");
    SnDummyFilesystem {
        root: SnDummyNode {
            node_type: SnVfsType::Dir,
            name: "",
            contents: None,
            children: Some(RwLock::new(BTreeMap::from([(
                "shinosawa",
                SnDummyNode {
                    node_type: SnVfsType::Dir,
                    name: "shinosawa",
                    contents: None,
                    children: Some(RwLock::new(BTreeMap::from([(
                        "system",
                        SnDummyNode {
                            node_type: SnVfsType::Dir,
                            name: "system",
                            contents: None,
                            children: Some(RwLock::new(BTreeMap::from([(
                                "kotono",
                                SnDummyNode {
                                    node_type: SnVfsType::File,
                                    name: "kotono",
                                    contents: Some(RwLock::new(Box::new([0 as u8; 30]))),
                                    children: None,
                                },
                            )]))),
                        },
                    )]))),
                },
            )]))),
        },
    }
}
