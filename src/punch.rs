//! Create files of a given size.
//!
//!

// Stop compilation on non unix systems
#[cfg(not(unix))]
compile_error!("This program is only supported on Unix systems");

use std::fs::File;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::usize;

use bytesize::ByteSize;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(version=env!("CARGO_PKG_VERSION"), author="Matheus Xavier <mxavier@neonimp.com>", about)]
struct PunchArgs {
    /// The file to create
    file: String,
    /// The size of the file to create (e.g. 1G, 1GiB, 1GB, 1GiB)
    size: String,
    /// Do not use fallocate(2), posix_fallocate(3) or ftruncate(2) instead write zeros to the file.
    #[clap(short = 'S', long, default_value = "false")]
    no_syscall: bool,
    /// Set file permissions ragardless of the umask
    #[clap(long)]
    permissions: Option<u32>,
}

fn main() -> std::io::Result<()> {
    // parse the command line arguments
    let args = PunchArgs::parse();

    // parse the size into bytes
    let size = match &args.size.parse::<ByteSize>() {
        Ok(size) => size.0,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // open the file
    let mut file = File::create(&args.file)?;
    // if permissions are requested, set them
    if let Some(iperm) = args.permissions {
        let metadata = file.metadata()?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(iperm);
    }

    // if the use_syscall flag is set, try to use fallocate(2), posix_fallocate(3) or ftruncate(2)
    if !args.no_syscall {
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::io::AsRawFd;
            let fd = file.as_raw_fd();
            let ret = unsafe { libc::fallocate(fd, 0, 0, size as i64) };
            if ret == 0 {
                return Ok(());
            }
        }
        #[cfg(target_os = "freebsd")]
        {
            use std::os::unix::io::AsRawFd;
            let fd = file.as_raw_fd();
            let ret = unsafe { libc::posix_fallocate(fd, 0, size as i64) };
            if ret == 0 {
                return Ok(());
            }
        }
        #[cfg(any(
            target_os = "macos",
            target_os = "ios",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            use std::os::unix::io::AsRawFd;
            let fd = file.as_raw_fd();
            let ret = unsafe { libc::ftruncate(fd, size as i64) };
            if ret == 0 {
                return Ok(());
            }
        }
    } else {
        // write zeros to the file
        let buf = vec![0; 1024 * 1024];
        let mut written = 0_usize;
        let mut buf_writer = std::io::BufWriter::new(&mut file);
        while let Ok(n) = buf_writer.write(&buf) {
            written += n;
            if written >= size as usize {
                break;
            }
        }
    }

    Ok(())
}
