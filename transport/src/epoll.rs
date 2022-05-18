use crate::libc;
use libc::epoll_event;

use libc::{O_CLOEXEC};
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};



macro_rules! syscall {
    ($func_name: ident ( $($arg: expr),* $(,)* ) ) => {
        {
            let result = unsafe { libc::$func_name($($arg,)* ) };
            if result == -1 { Err(std::io::Error::last_os_error()) } else { Ok(result) }
        }
    }
}

pub struct Registry {
    epoll_fd: RawFd,
}

impl Registry {
    const READ_FLAGS: libc::c_int = libc::EPOLLIN;
    const WRITE_FLAGS: libc::c_int = libc::EPOLLOUT;
    const READ_KEY: u64 = 0;
    const WRITE_KEY: u64 = 1;

    pub fn new() -> io::Result<Self> {
        let epoll_fd = syscall!(epoll_create1(O_CLOEXEC)).expect("cannot create an epoll");
        Ok(Self { epoll_fd })
    }

    const fn read_event() -> epoll_event {
        epoll_event { events: Registry::READ_FLAGS as u32, u64: Registry::READ_KEY }
    }

    const fn write_event() -> epoll_event {
        epoll_event { events: Registry::READ_FLAGS as u32, u64: Registry::READ_KEY }
    }

    pub fn register_read(&self, fd: RawFd) {
        let mut
        syscall!(epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_ADD, fd, ))
    }
}

