use std::io::{self, Write, Cursor};
use std::fmt;
use std::str::from_utf8;
use std::ffi::CString;
use std::path::Path;

use libc::{c_void, uid_t, gid_t, mode_t, c_char};
use libc::mount;
use nix::mount::{MsFlags};

use {OSError, Error};
use util::{path_to_cstring, as_path};
use explain::{Explainable, exists, user};


#[derive(Debug, Clone, Copy)]
enum Size {
    Auto,
    Bytes(usize),
    Blocks(usize),
}

/// A tmpfs mount definition
///
/// By default tmpfs is mounted with nosuid,nodev
#[derive(Debug, Clone)]
pub struct Tmpfs {
    target: CString,
    size: Size,
    nr_inodes: Option<usize>,
    mode: Option<mode_t>,
    uid: Option<uid_t>,
    gid: Option<gid_t>,
    flags: MsFlags,
}

impl Tmpfs {
    /// New tmpfs mount point with target path and default settngs
    pub fn new<P: AsRef<Path>>(path: P) -> Tmpfs {
        Tmpfs {
            target: path_to_cstring(path.as_ref()),
            size: Size::Auto,
            nr_inodes: None,
            mode: None,
            uid: None,
            gid: None,
            flags: MsFlags::MS_NOSUID|MsFlags::MS_NODEV,
        }
    }
    /// Set size in bytes
    pub fn size_bytes(mut self, size: usize) -> Tmpfs {
        self.size = Size::Bytes(size);
        self
    }
    /// Set size in blocks of PAGE_CACHE_SIZE
    pub fn size_blocks(mut self, size: usize) -> Tmpfs {
        self.size = Size::Blocks(size);
        self
    }
    /// Maximum number of inodes
    pub fn nr_inodes(mut self, num: usize) -> Tmpfs {
        self.nr_inodes = Some(num);
        self
    }
    /// Set initial permissions of the root directory
    pub fn mode(mut self, mode: mode_t) -> Tmpfs {
        self.mode = Some(mode);
        self
    }
    /// Set initial owner of the root directory
    pub fn uid(mut self, uid: uid_t) -> Tmpfs {
        self.uid = Some(uid);
        self
    }
    /// Set initial group of the root directory
    pub fn gid(mut self, gid: gid_t) -> Tmpfs {
        self.gid = Some(gid);
        self
    }

    fn format_options(&self) -> Vec<u8> {
        let mut cur = Cursor::new(Vec::new());
        match self.size {
            Size::Auto => {}
            Size::Bytes(x) => write!(cur, "size={}", x).unwrap(),
            Size::Blocks(x) => write!(cur, "nr_blocks={}", x).unwrap(),
        }
        if let Some(inodes) = self.nr_inodes {
            if cur.position() != 0 {
                cur.write(b",").unwrap();
            }
            write!(cur, "nr_inodes={}", inodes).unwrap();
        }
        if let Some(mode) = self.mode {
            if cur.position() != 0 {
                cur.write(b",").unwrap();
            }
            write!(cur, "mode=0{:04o}", mode).unwrap();
        }
        if let Some(uid) = self.uid {
            if cur.position() != 0 {
                cur.write(b",").unwrap();
            }
            write!(cur, "uid={}", uid).unwrap();
        }
        if let Some(gid) = self.gid {
            if cur.position() != 0 {
                cur.write(b",").unwrap();
            }
            write!(cur, "gid={}", gid).unwrap();
        }
        return cur.into_inner();
    }

    /// Mount the tmpfs
    pub fn bare_mount(self) -> Result<(), OSError> {
        let mut options = self.format_options();
        options.push(0);
        let rc = unsafe { mount(
                b"tmpfs\0".as_ptr() as *const c_char,
                self.target.as_ptr(),
                b"tmpfs\0".as_ptr() as *const c_char,
                self.flags.bits(),
                options.as_ptr() as *const c_void) };
        if rc < 0 {
            Err(OSError::from_io(io::Error::last_os_error(), Box::new(self)))
        } else {
            Ok(())
        }
    }

    /// Mount the tmpfs and explain error immediately
    pub fn mount(self) -> Result<(), Error> {
        self.bare_mount().map_err(OSError::explain)
    }
}

impl fmt::Display for Tmpfs {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let opts = self.format_options();
        write!(fmt, "tmpfs {} -> {:?}", from_utf8(&opts).unwrap(),
            as_path(&self.target))
    }
}

impl Explainable for Tmpfs {
    fn explain(&self) -> String {
        [
            format!("target: {}", exists(as_path(&self.target))),
            format!("{}", user()),
        ].join(", ")
    }
}


mod test {
    #[cfg(test)]
    use super::Tmpfs;

    #[test]
    fn test_tmpfs_options() {
        let fs = Tmpfs::new("/tmp")
            .size_bytes(1 << 20)
            .nr_inodes(1024)
            .mode(0o1777)
            .uid(1000)
            .gid(1000);

        assert_eq!(fs.format_options(),
            "size=1048576,nr_inodes=1024,mode=01777,uid=1000,gid=1000".as_bytes())
    }
}
