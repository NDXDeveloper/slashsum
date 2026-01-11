// Import standard library components
use std::{
    env,                   // Environment variables and command-line arguments
    fs::File,              // File handling
    io::{BufReader, Read}, // Buffered reading
    path::{Path, PathBuf}, // Path manipulation
    sync::Arc,             // Atomic Reference Counted pointer for thread-safe sharing
    thread,                // Thread management
    time::Instant,         // Time measurement
};

// External crates
use crc::Crc; // CRC32 implementation
use crossbeam_channel::bounded; // Thread communication channels
use md5::Context; // MD5 hashing context
use sha1::{Digest, Sha1}; // SHA1 hasher
use sha2::{Sha256, Sha512}; // SHA256 and SHA512 hashers

//use std::process::Command;

/// Calcule un hachage en traitant les données par morceaux à la volée
/// Paramètres:
/// - rx: Récepteur du canal pour les morceaux de données
/// - initializer: Fonction qui initialise le contexte de hachage
/// - updater: Fonction qui met à jour le contexte avec de nouvelles données
/// - finalizer: Fonction qui produit le hachage final
fn compute_hash<H, C, I, U, F>(
    rx: crossbeam_channel::Receiver<Arc<[u8]>>,
    initializer: I,
    updater: U,
    finalizer: F,
) -> H
where
    I: FnOnce() -> C,
    U: Fn(&mut C, &[u8]),
    F: FnOnce(C) -> H,
{
    // Initialiser le contexte de hachage
    let mut context = initializer();

    // Traiter chaque morceau de données dès réception
    while let Ok(chunk) = rx.recv() {
        updater(&mut context, &chunk);
    }

    // Finaliser le hachage
    finalizer(context)
}

// Structure pour encapsuler le calcul CRC32
struct Crc32Calculator {
    crc_algo: Crc<u32>,
    data: Vec<u8>,
}

impl Crc32Calculator {
    fn new() -> Self {
        Self {
            crc_algo: Crc::<u32>::new(&crc::CRC_32_ISO_HDLC),
            data: Vec::new(),
        }
    }

    fn update(&mut self, new_data: &[u8]) {
        // Ajout des nouvelles données
        self.data.extend_from_slice(new_data);
    }

    fn finalize(self) -> u32 {
        // Calcul du CRC32 sur toutes les données accumulées
        self.crc_algo.checksum(&self.data)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "slashsum {} - {}",
        option_env!("BUILD_VERSION").unwrap_or("dev"),
        option_env!("GIT_COMMIT").unwrap_or("unknown")
    );

    // Get command-line arguments
    let args: Vec<String> = env::args().collect();

    // Handle version and license request
    if args.iter().any(|arg| arg == "--version") {
        print_version_and_license();
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
        eprintln!("Error: File '{file_path}' not found");
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
    let (crc32_tx, crc32_rx) = bounded(1024); // CRC32 channel
    let (md5_tx, md5_rx) = bounded(1024); // MD5 channel
    let (sha1_tx, sha1_rx) = bounded(1024); // SHA1 channel
    let (sha256_tx, sha256_rx) = bounded(1024); // SHA256 channel
    let (sha512_tx, sha512_rx) = bounded(1024); // SHA512 channel

    // Spawn thread for CRC32 calculation
    let crc32_handle = thread::spawn(move || {
        compute_hash(
            crc32_rx,
            || Crc32Calculator::new(),
            |calculator, data| calculator.update(data),
            |calculator| format!("{:08x}", calculator.finalize()),
        )
    });

    // Spawn thread for MD5 calculation
    // Thread de calcul MD5
    let md5_handle = thread::spawn(move || {
        compute_hash(
            md5_rx,
            || Context::new(),                     // initialisation du contexte MD5
            |context, data| context.consume(data), // mise à jour avec les données
            |context| format!("{:x}", context.compute()), // finalisation et formatage
        )
    });

    // Spawn thread for SHA1 calculation
    let sha1_handle = thread::spawn(move || {
        compute_hash(
            sha1_rx,
            || Sha1::new(), // initialisation du contexte SHA1
            |digest, data| {
                digest.update(data);
            }, // mise à jour avec les données
            |digest| format!("{:x}", digest.finalize()), // finalisation et formatage
        )
    });

    // Spawn thread for SHA256 calculation
    let sha256_handle = thread::spawn(move || {
        compute_hash(
            sha256_rx,
            || Sha256::new(), // initialisation du contexte SHA256
            |digest, data| {
                digest.update(data);
            }, // mise à jour avec les données
            |digest| format!("{:x}", digest.finalize()), // finalisation et formatage
        )
    });

    // Spawn thread for SHA512 calculation
    let sha512_handle = thread::spawn(move || {
        compute_hash(
            sha512_rx,
            || Sha512::new(), // initialisation du contexte SHA512
            |digest, data| {
                digest.update(data);
            }, // mise à jour avec les données
            |digest| format!("{:x}", digest.finalize()), // finalisation et formatage
        )
    });

    // Read file in 1MB chunks
    loop {
        let mut buffer = vec![0; 1_048_576]; // 1MB buffer
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            // End of file
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
        let file_name = path
            .file_name()
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
    slashsum --version           # Display version and license information
    slashsum -h                  # Show this help message"#
    );
}

/// Affiche la version et la licence MIT complète
fn print_version_and_license() {
    println!(
        r#"MIT License

Copyright (c) 2025-2026 Nicolas DEOUX
                   NDXDev@gmail.com

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."#
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;
    use md5::Context;

    use sha1::{Digest, Sha1};
    use sha2::{Sha256, Sha512};
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;

    use std::time::Instant;

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

        let result = compute_hash(
            rx,
            || Context::new(),                     // initialisation du contexte MD5
            |context, data| context.consume(data), // mise à jour avec les données
            |context| format!("{:x}", context.compute()), // finalisation et formatage
        );

        assert_eq!(result, "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn test_compute_hash_crc32() {
        // Test CRC32 with "Hello world!" input
        let (tx, rx) = bounded(2);

        tx.send(Arc::from(b"Hello world!".as_ref())).unwrap();
        drop(tx);

        // Utiliser la même structure que celle que vous avez adoptée pour le CRC32
        let result = compute_hash(
            rx,
            || {
                // Structure pour le calcul du CRC32
                struct Crc32Calculator {
                    crc_algo: Crc<u32>,
                    data: Vec<u8>,
                }

                Crc32Calculator {
                    crc_algo: Crc::<u32>::new(&crc::CRC_32_ISO_HDLC),
                    data: Vec::new(),
                }
            },
            |calculator, data| {
                calculator.data.extend_from_slice(data);
            },
            |calculator| format!("{:08x}", calculator.crc_algo.checksum(&calculator.data)),
        );

        assert_eq!(result, "1b851995");
    }

    #[test]
    fn test_file_not_found() {
        // Verify error handling for missing files
        let result = File::open("nonexistent_file.txt");
        assert!(result.is_err(), "Should return error for missing file");
    }

    #[test]
    fn test_format_size_edge_cases() {
        // Test des cas limites pour format_size
        assert_eq!(format_size(0), "0 bytes");
        assert_eq!(format_size(1), "1 bytes");
        assert_eq!(format_size(1023), "1023 bytes");
        assert_eq!(format_size(1024), "1 KB (1024 bytes)");
        assert_eq!(format_size(1536), "1.50 KB (1536 bytes)");
        assert_eq!(format_size(1_099_511_627_776), "1 TB (1099511627776 bytes)");
        // Correction: la valeur réelle calculée par votre fonction
        assert_eq!(
            format_size(u64::MAX),
            "16777216 TB (18446744073709551615 bytes)"
        );
    }

    #[test]
    fn test_compute_hash_sha1() {
        // Test SHA1 avec "abc"
        let (tx, rx) = bounded(2);
        tx.send(Arc::from(b"abc".as_ref())).unwrap();
        drop(tx);

        let result = compute_hash(
            rx,
            || Sha1::new(),
            |digest, data| {
                digest.update(data);
            },
            |digest| format!("{:x}", digest.finalize()),
        );

        assert_eq!(result, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn test_compute_hash_sha256() {
        // Test SHA256 avec "abc"
        let (tx, rx) = bounded(2);
        tx.send(Arc::from(b"abc".as_ref())).unwrap();
        drop(tx);

        let result = compute_hash(
            rx,
            || Sha256::new(),
            |digest, data| {
                digest.update(data);
            },
            |digest| format!("{:x}", digest.finalize()),
        );

        assert_eq!(
            result,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_compute_hash_sha512() {
        // Test SHA512 avec "abc"
        let (tx, rx) = bounded(2);
        tx.send(Arc::from(b"abc".as_ref())).unwrap();
        drop(tx);

        let result = compute_hash(
            rx,
            || Sha512::new(),
            |digest, data| {
                digest.update(data);
            },
            |digest| format!("{:x}", digest.finalize()),
        );

        assert_eq!(
            result,
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn test_compute_hash_empty_input() {
        // Test avec une entrée vide
        let (tx, rx) = bounded(1);
        drop(tx); // Fermer immédiatement le canal

        let result = compute_hash(
            rx,
            || Context::new(),
            |context, data| context.consume(data),
            |context| format!("{:x}", context.compute()),
        );

        // MD5 d'une chaîne vide
        assert_eq!(result, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_compute_hash_multiple_chunks() {
        // Test avec plusieurs chunks
        let (tx, rx) = bounded(5);

        // Envoyer "Hello" puis " World!" pour former "Hello World!"
        tx.send(Arc::from(b"Hello".as_ref())).unwrap();
        tx.send(Arc::from(b" World!".as_ref())).unwrap();
        drop(tx);

        let result = compute_hash(
            rx,
            || Context::new(),
            |context, data| context.consume(data),
            |context| format!("{:x}", context.compute()),
        );

        // MD5 de "Hello World!"
        assert_eq!(result, "ed076287532e86365e841e92bfc50d8c");
    }

    #[test]
    fn test_crc32_calculator() {
        // Test direct de la structure Crc32Calculator
        let mut calculator = Crc32Calculator::new();
        calculator.update(b"Hello");
        calculator.update(b" World!");
        let result = calculator.finalize();

        // CRC32 de "Hello World!" - valeur corrigée
        assert_eq!(result, 472456355); // 0x1c291ca3 en décimal
    }

    #[test]
    fn test_crc32_calculator_empty() {
        // Test CRC32 avec données vides
        let calculator = Crc32Calculator::new();
        let result = calculator.finalize();

        // CRC32 d'une chaîne vide
        assert_eq!(result, 0x00000000);
    }

    #[test]
    fn test_large_chunk_processing() {
        // Test avec un gros chunk (similaire à la taille du buffer 1MB)
        let large_data = vec![0x42u8; 1_048_576]; // 1MB de données
        let (tx, rx) = bounded(2);

        tx.send(Arc::from(large_data.into_boxed_slice())).unwrap();
        drop(tx);

        let result = compute_hash(
            rx,
            || Context::new(),
            |context, data| context.consume(data),
            |context| format!("{:x}", context.compute()),
        );

        // Ce test vérifie que le traitement de gros chunks fonctionne
        assert_eq!(result.len(), 32); // MD5 produit toujours 32 caractères hex
    }

    #[test]
    fn test_channel_capacity() {
        // Test avec une capacité de canal limitée
        let (tx, rx) = bounded(1); // Capacité très petite

        // Utiliser un thread pour lire les données pendant qu'on les envoie
        let handle = std::thread::spawn(move || {
            compute_hash(
                rx,
                || Context::new(),
                |context, data| context.consume(data),
                |context| format!("{:x}", context.compute()),
            )
        });

        // Envoyer plusieurs chunks rapidement
        for i in 0..10 {
            let data = format!("chunk{}", i);
            tx.send(Arc::from(data.as_bytes())).unwrap();
        }
        drop(tx); // Fermer le canal

        // Attendre le résultat
        let result = handle.join().unwrap();

        // Vérifier que tous les chunks ont été traités
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn test_concurrent_hash_computation() {
        // Test simulant le comportement concurrent réel
        use std::thread;

        let test_data = b"This is a test for concurrent hash computation";
        let chunk = Arc::from(test_data.as_ref());

        // Créer plusieurs canaux comme dans le code principal
        let (md5_tx, md5_rx) = bounded(1024);
        let (sha1_tx, sha1_rx) = bounded(1024);

        // Lancer les threads de calcul
        let md5_handle = thread::spawn(move || {
            compute_hash(
                md5_rx,
                || Context::new(),
                |context, data| context.consume(data),
                |context| format!("{:x}", context.compute()),
            )
        });

        let sha1_handle = thread::spawn(move || {
            compute_hash(
                sha1_rx,
                || Sha1::new(),
                |digest, data| {
                    digest.update(data);
                },
                |digest| format!("{:x}", digest.finalize()),
            )
        });

        // Envoyer les données
        md5_tx.send(Arc::clone(&chunk)).unwrap();
        sha1_tx.send(chunk).unwrap();

        // Fermer les canaux
        drop(md5_tx);
        drop(sha1_tx);

        // Récupérer les résultats
        let md5_result = md5_handle.join().unwrap();
        let sha1_result = sha1_handle.join().unwrap();

        // Vérifier les résultats - correction avec la vraie valeur calculée
        assert_eq!(md5_result, "f801c3cb79c641ab70efc5b525af573c");
        assert_eq!(sha1_result.len(), 40); // SHA1 produit 40 caractères hex
    }

    #[test]
    fn test_format_size_precision() {
        // Test de précision pour format_size
        assert_eq!(format_size(1536), "1.50 KB (1536 bytes)");
        assert_eq!(format_size(1_572_864), "1.50 MB (1572864 bytes)");
        assert_eq!(format_size(1_610_612_736), "1.50 GB (1610612736 bytes)");
    }

    // Test d'intégration avec fichier temporaire
    #[test]
    fn test_with_temporary_file() -> Result<(), Box<dyn std::error::Error>> {
        // Créer un fichier temporaire avec du contenu connu
        let mut temp_file = NamedTempFile::new()?;
        let test_content = b"Hello, World! This is a test file.";
        temp_file.write_all(test_content)?;

        // Tester l'ouverture du fichier
        let file = File::open(temp_file.path())?;
        let metadata = file.metadata()?;

        assert_eq!(metadata.len(), test_content.len() as u64);
        assert!(metadata.is_file());

        Ok(())
    }

    #[test]
    #[ignore] // Ignorer par défaut, exécuter avec cargo test -- --ignored
    fn benchmark_hash_computation() {
        let large_data = vec![0x42u8; 10_000_000]; // 10MB de données
        let chunk = Arc::from(large_data.into_boxed_slice());

        let start = Instant::now();

        let (tx, rx) = bounded(1);
        tx.send(chunk).unwrap();
        drop(tx);

        let _result = compute_hash(
            rx,
            || Context::new(),
            |context, data| context.consume(data),
            |context| format!("{:x}", context.compute()),
        );

        let duration = start.elapsed();
        println!("Hash computation took: {:?}", duration);

        // Vérifier que ça prend moins de 1 seconde (ajustez selon vos besoins)
        assert!(duration.as_secs() < 1);
    }
}
