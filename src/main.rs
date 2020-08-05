extern crate pty;
extern crate nix;
extern crate libc;

#[macro_use]
extern crate simple_error;

use std::os::unix::io::FromRawFd;
use std::os::unix::io::AsRawFd;
use simple_error::SimpleError;
use std::io::Stdin;

fn main() -> std::result::Result<(), Box<dyn std::error::Error> >  {
    // 1 -- First get the original terminal attributes
    let stdin = std::io::stdin();
    let orig_attr = nix::sys::termios::tcgetattr(stdin.as_raw_fd())?;

    let window : libc::winsize = unsafe {
        get_window(&stdin)?
    };

    println!("{:?}", window);

    let fork_result = nix::pty::forkpty(Some(&window), Some(&orig_attr))?;

    let master_file : std::fs::File = unsafe {
        std::fs::File::from_raw_fd(fork_result.master)
    };

    // this should print '/dev/ptmx' as the master device
    // https://linux.die.net/man/4/ptmx
    // Each file descriptor obtained by opening /dev/ptmx
    // is an independent PTM with its own associated pseudoterminal slaves (PTS)
    println!("{:?}", master_file);

    match fork_result.fork_result {

        // the child simply exec's into a shell
        nix::unistd::ForkResult::Child => {
            println!("Executing child.");

            let shell = std::env::var_os("SHELL")
                .unwrap_or(std::ffi::OsString::from("/bin/sh"))
                .into_string().expect("We expectd to convert from OString to String");

            let c_str = std::ffi::CString::new(shell).expect("CString::new failed");
            nix::unistd::execv(&c_str, &[]);
        }

        // the parent will relay data between terminal and pty master
        nix::unistd::ForkResult::Parent { child, .. } => {
            println!("Executing parent.");
            println!("{:?}", child);

            let output_file = std::fs::File::create("typescript")?;

            let mut tty = nix::sys::termios::tcgetattr(stdin.as_raw_fd())?;
            nix::sys::termios::cfmakeraw(&mut tty);

            let mut in_fds = nix::sys::select::FdSet::new();
            in_fds.insert(stdin.as_raw_fd());
            in_fds.insert(fork_result.master);

            loop {

                let number_ready = nix::sys::select::select(None, Some(&mut in_fds), None, None, None)?;

            }
        }
    }


    Ok(())
}

unsafe fn get_window(stdin: &Stdin) -> Result<libc::winsize, SimpleError> {
    let mut window: std::mem::MaybeUninit<libc::winsize> = std::mem::MaybeUninit::uninit();
    let result = libc::ioctl(stdin.as_raw_fd(), libc::TIOCGWINSZ, window.as_mut_ptr());
    if result < 0 {
        bail!("Failed to get window size.");
    }
    Ok(window.assume_init())
}

