use crate::{libc, util};
use libc::epoll_event;

use std::collections::HashMap;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::Duration;


/* TODO: expose EventType instead of epoll_event (in registry' await_events())  */


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
            EventType::Read =>  Self::read_event(),
            EventType::Write => Self::write_event(),
        }
    }

    const fn read_event() -> epoll_event {
        epoll_event { events: Registry::READ_FLAGS as u32, u64: Registry::READ_KEY }
    }

    const fn write_event() -> epoll_event {
        epoll_event { events: Registry::READ_FLAGS as u32, u64: Registry::READ_KEY }
    }
}

pub enum Notification<'a> {
    Timeout,
    Events(&'a [epoll_event]),
}


pub struct Registry {
    epoll_fd: RawFd,
    events: Vec<epoll_event>,
    instances: HashMap<RawFd, HashMap<EventType, epoll_event>>,
    timeout: Duration,
}

impl Registry {
    const READ_FLAGS: libc::c_int = libc::EPOLLIN;
    const WRITE_FLAGS: libc::c_int = libc::EPOLLOUT;
    const READ_KEY: u64 = 0;
    const WRITE_KEY: u64 = 1;
    const DEFAULT_TIMEOUT: Duration = Duration::from_millis(1500);

    pub fn new() -> io::Result<Self> {
        Self::with_timeout(Self::DEFAULT_TIMEOUT)
    }

    pub fn with_timeout(timeout: Duration) -> io::Result<Self> {
        let epoll_fd = syscall!(epoll_create1(libc::O_CLOEXEC)).expect("cannot create an epoll");
        Ok(Self { epoll_fd, events: Vec::new(), instances: HashMap::new(), timeout })
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

    pub fn await_events(&mut self) -> Notification {
        self.events.clear();
        let res = syscall!(
            epoll_wait(
                self.epoll_fd,
                self.events.as_mut_ptr() as *mut epoll_event,
                self.events.len() as i32,
                self.timeout.as_millis() as libc::c_int,
            )
        ).map_err(|err| {
            util::fail_with_message(format!("error during epoll wait: {err}"));
        }).unwrap();

        // safety: since events was empty before epoll_wait syscall the length of self.events
        // after should be exactly res (assuming kernel is correct).
        unsafe { self.events.set_len(res as usize); }
        if self.events.len() == 0 {
            Notification::Timeout
        } else {
            Notification::Events(&self.events[..])
        }
    }
}
