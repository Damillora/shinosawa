use alloc::{boxed::Box, collections::BTreeMap};

use crate::printk;

use super::vfs::{SnVfsError, SnVfsFilesystem, SnVfsNode};


struct SnDummyNode {
    contents: Option<Box<[u8]>>,
    children: Option<BTreeMap<&'static str, SnDummyNode>>,
}
impl SnVfsNode for SnDummyNode {
    fn is_file(&self) -> bool {
        self.contents.is_some() && self.children.is_none()
    }

    fn is_dir(&self) -> bool {
        self.contents.is_none() && self.children.is_some()
    }
    
    fn read(&self, buf: &mut [u8]) -> Result<usize, SnVfsError> {
        if let Some(content) = &self.contents {
            buf[0..content.len()].copy_from_slice(&content);

            return Ok(content.len());
        }

        Err(SnVfsError::ReadError)
    }

    fn len(&self) -> usize {
        if let Some(content) = &self.contents {
            return content.len();
        }

        0
    }
}
pub struct SnDummyFilesystem {
    root: SnDummyNode,
}

impl SnVfsFilesystem for SnDummyFilesystem {
    fn startup(&self) {
        
    }
}

pub fn new_example_filesystem() -> SnDummyFilesystem {
    printk!("fs::dummy: creating a sample rootfs");
    SnDummyFilesystem {
        root: SnDummyNode { contents: None, children: Some(BTreeMap::from([
            ("shinosawa", SnDummyNode {
                contents: None,
                children: Some(BTreeMap::from([
                    ("system", SnDummyNode {
                        contents: None,
                        children: Some(BTreeMap::from([
                            ("servman", SnDummyNode {
                                contents: Some(Box::new([0 as u8; 30])),
                                children: None,
                            })
                        ]))
                    }),
                ])),
            })
        ])) }
    }
}