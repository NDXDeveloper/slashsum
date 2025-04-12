// Import standard library components
use std::{
    env, // Environment variables and command-line arguments
    fs::File, // File handling
    io::{BufReader, Read}, // Buffered reading
    path::{Path, PathBuf}, // Path manipulation
    sync::Arc, // Atomic Reference Counted pointer for thread-safe sharing
    thread, // Thread management
    time::Instant, // Time measurement
};

// External crates
use crossbeam_channel::bounded; // Thread communication channels
use crc::Crc; // CRC32 implementation
use md5::Context; // MD5 hashing context
use sha1::{Digest, Sha1}; // SHA1 hasher
use sha2::{Sha256, Sha512}; // SHA256 and SHA512 hashers

/// Computes a hash by aggregating data chunks from a channel
/// Parameters:
/// - rx: Receiver end of the channel for data chunks
/// - finalizer: Function that processes the complete data and returns the hash
fn compute_hash<H, F>(rx: crossbeam_channel::Receiver<Arc<[u8]>>, finalizer: F) -> H
where
    F: FnOnce(Vec<u8>) -> H,
{
    let mut buffer = Vec::new();

    // Collect all data chunks until channel closes
    while let Ok(chunk) = rx.recv() {
        buffer.extend_from_slice(&chunk);
    }

    // Process complete data with the provided hash function
    finalizer(buffer)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get command-line arguments
    let args: Vec<String> = env::args().collect();

    // Handle version request
    if args.iter().any(|arg| arg == "--version") {
        println!("slashsum version {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Handle help request
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help();
        return Ok(());
    }

    // Validate argument count
    if args.len() < 2 || args.len() > 3 {
        eprintln!("Error: Invalid number of arguments");
        eprintln!("Use -h or --help for usage instructions");
        std::process::exit(1);
    }

    // Check for --save flag
    let save_flag = if args.len() == 3 {
        if args[2] != "--save" {
            eprintln!("Error: Invalid option '{}'", args[2]);
            std::process::exit(1);
        }
        true
    } else {
        false
    };

    // Validate input file exists
    let file_path = &args[1];
    if !Path::new(file_path).exists() {
        eprintln!("Error: File '{}' not found", file_path);
        eprintln!("Use -h or --help for usage instructions");
        std::process::exit(1);
    }

    // Start performance timer
    let start_time = Instant::now();

    // Open file and create buffered reader
    let file = File::open(file_path)?;
    let metadata = file.metadata()?;
    let size = metadata.len();
    let mut reader = BufReader::with_capacity(1_048_576, file); // 1MB buffer

    // Create communication channels for each hash algorithm
    let (crc32_tx, crc32_rx) = bounded(1024);  // CRC32 channel
    let (md5_tx, md5_rx) = bounded(1024);      // MD5 channel
    let (sha1_tx, sha1_rx) = bounded(1024);    // SHA1 channel
    let (sha256_tx, sha256_rx) = bounded(1024);// SHA256 channel
    let (sha512_tx, sha512_rx) = bounded(1024);// SHA512 channel

    // Spawn thread for CRC32 calculation
    let crc32_handle = thread::spawn(move || {
        compute_hash(crc32_rx, |data| {
            let crc = Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
            format!("{:08x}", crc.checksum(&data)) // 8-digit hexadecimal
        })
    });

    // Spawn thread for MD5 calculation
    let md5_handle = thread::spawn(move || {
        compute_hash(md5_rx, |data| {
            let mut context = Context::new();
            context.consume(&data);
            format!("{:x}", context.compute())
        })
    });

    // Spawn thread for SHA1 calculation
    let sha1_handle = thread::spawn(move || {
        compute_hash(sha1_rx, |data| {
            format!("{:x}", Sha1::new().chain_update(&data).finalize())
        })
    });

    // Spawn thread for SHA256 calculation
    let sha256_handle = thread::spawn(move || {
        compute_hash(sha256_rx, |data| {
            format!("{:x}", Sha256::new().chain_update(&data).finalize())
        })
    });

    // Spawn thread for SHA512 calculation
    let sha512_handle = thread::spawn(move || {
        compute_hash(sha512_rx, |data| {
            format!("{:x}", Sha512::new().chain_update(&data).finalize())
        })
    });

    // Read file in 1MB chunks
    loop {
        let mut buffer = vec![0; 1_048_576]; // 1MB buffer
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 { // End of file
            break;
        }
        buffer.truncate(bytes_read);
        let chunk = Arc::from(buffer.into_boxed_slice());

        // Send chunk to all hash channels
        crc32_tx.send(Arc::clone(&chunk))?;
        md5_tx.send(Arc::clone(&chunk))?;
        sha1_tx.send(Arc::clone(&chunk))?;
        sha256_tx.send(Arc::clone(&chunk))?;
        sha512_tx.send(chunk)?; // Final send uses original Arc
    }

    // Close all transmission channels
    drop(crc32_tx);
    drop(md5_tx);
    drop(sha1_tx);
    drop(sha256_tx);
    drop(sha512_tx);

    // Collect results from all threads
    let crc32 = crc32_handle.join().map_err(|_| "Thread CRC32 error")?;
    let md5 = md5_handle.join().map_err(|_| "Thread MD5 error")?;
    let sha1 = sha1_handle.join().map_err(|_| "Thread SHA1 error")?;
    let sha256 = sha256_handle.join().map_err(|_| "Thread SHA256 error")?;
    let sha512 = sha512_handle.join().map_err(|_| "Thread SHA512 error")?;

    // Format final output
    let output = format!(
        "File: {}\nSize:    {}\nCRC32:   {}\nMD5:   {}\nSHA1:  {}\nSHA256: {}\nSHA512: {}\nTime:  {:.2?}",
        file_path,
        format_size(size),
        crc32,
        md5,
        sha1,
        sha256,
        sha512,
        start_time.elapsed()
    );
    println!("{}", output);

    // Handle --save flag
    if save_flag {
        let path = Path::new(file_path);
        let file_name = path.file_name()
            .ok_or("Invalid file name")?
            .to_str()
            .ok_or("File name is not valid UTF-8")?;
        let mut output_path = PathBuf::from(path);
        output_path.set_file_name(format!("{}.checksum", file_name));
        std::fs::write(&output_path, output)?;

        println!("Checksums saved to: {}", output_path.display());
    } else {
        //println!("{}", output);
    }

    Ok(())
}

/// Converts byte count to human-readable format
/// Example: 1024 → "1 KB (1024 bytes)"
fn format_size(size_bytes: u64) -> String {
    const UNITS: [&str; 5] = ["bytes", "KB", "MB", "GB", "TB"];
    let mut size = size_bytes as f64;
    let mut unit_index = 0;

    // Find appropriate unit
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    // Format based on unit type
    let formatted = if unit_index == 0 {
        format!("{} {}", size_bytes, UNITS[unit_index]) // Bytes
    } else if size.fract() == 0.0 {
        format!("{} {}", size as u64, UNITS[unit_index]) // Whole number
    } else {
        format!("{:.2} {}", size, UNITS[unit_index]) // Decimal
    };

    // Add original byte count for converted units
    if unit_index > 0 {
        format!("{} ({size_bytes} bytes)", formatted)
    } else {
        formatted
    }
}

/// Displays help information
fn print_help() {
    println!(
        r#"
Slashsum - Calculate multiple checksums simultaneously

USAGE:
    slashsum <FILE> [OPTIONS]

OPTIONS:
    --save       Save checksums to a .checksum file
    -h, --help   Print help information
    --version    Print version information

EXAMPLES:
    slashsum file.txt            # Calculate and display checksums
    slashsum file.txt --save     # Save results to file.txt.checksum
    slashsum --version           # Display version information
    slashsum -h                  # Show this help message"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;
    use md5::Context;

    #[test]
    fn test_format_size() {
        // Test various size conversions
        assert_eq!(format_size(1_048_576), "1 MB (1048576 bytes)");
        assert_eq!(format_size(1_073_741_824), "1 GB (1073741824 bytes)");
        assert_eq!(format_size(3_145_728), "3 MB (3145728 bytes)");
    }

    #[test]
    fn test_compute_hash_md5() {
        // Test MD5 with known "abc" input
        let (tx, rx) = bounded(2);

        tx.send(Arc::from([0x61u8, 0x62, 0x63])).unwrap(); // "abc"
        drop(tx);

        let result = compute_hash(rx, |data| {
            let mut context = Context::new();
            context.consume(&data);
            format!("{:x}", context.compute())
        });

        assert_eq!(result, "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn test_compute_hash_crc32() {
        // Test CRC32 with "Hello world!" input
        let (tx, rx) = bounded(2);

        tx.send(Arc::from(b"Hello world!".as_ref())).unwrap();
        drop(tx);

        let result = compute_hash(rx, |data| {
            let crc = Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
            format!("{:08x}", crc.checksum(&data)) // 8-digit hexadecimal
        });

        assert_eq!(result, "1b851995");
    }

    #[test]
    fn test_file_not_found() {
        // Verify error handling for missing files
        let result = File::open("nonexistent_file.txt");
        assert!(result.is_err(), "Should return error for missing file");
    }
}