use crate::libc;
use libc::epoll_event;

use std::collections::HashMap;
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


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum EventType {
    Read,
    Write,
}

impl EventType {
    pub(self) const fn epoll_event(&self) -> epoll_event {
        match &self {
            EventType::Read =>  read_event(),
            EventType::Write => write_event(),
        }
    }

    const fn read_event() -> epoll_event {
        epoll_event { events: Registry::READ_FLAGS as u32, u64: Registry::READ_KEY }
    }

    const fn write_event() -> epoll_event {
        epoll_event { events: Registry::READ_FLAGS as u32, u64: Registry::READ_KEY }
    }
}


pub struct Registry {
    epoll_fd: RawFd,
    instances: HashMap<RawFd, HashMap<EventType, epoll_event>>,
}

impl Registry {
    const READ_FLAGS: libc::c_int = libc::EPOLLIN;
    const WRITE_FLAGS: libc::c_int = libc::EPOLLOUT;
    const READ_KEY: u64 = 0;
    const WRITE_KEY: u64 = 1;

    pub fn new() -> io::Result<Self> {
        let epoll_fd = syscall!(epoll_create1(libc::O_CLOEXEC)).expect("cannot create an epoll");
        Ok(Self { epoll_fd, instances: HashMap::new() })
    }

    /// Registers interest in `event_type` for `fd`.
    pub fn add_interest(&mut self, event_type: EventType, fd: impl AsRawFd) -> io::Result<()> {
        let fd = fd.as_raw_fd();
        let new_interest_epoll_event = event_type.epoll_event();
        self.instances.entry(fd)
            .and_modify(|interests| { interests.insert(event_type, new_interest_epoll_event); })
            .or_insert(HashMap::from([(event_type, new_interest_epoll_event)]));
        let event_args = self.instances.get_mut(&fd).unwrap().get_mut(&event_type).unwrap();
        syscall!(epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_ADD, fd, event_args))?;
        Ok(())
    }

    pub fn delete_interest(&mut self, event_type: EventType, fd: impl AsRawFd) -> io::Result<()> {
        let fd = fd.as_raw_fd();
        syscall!(epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut()))?;
        self.instances.entry(fd).and_modify(|interests| { interests.remove(&event_type); });
        Ok(())
    }
}
