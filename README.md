# Slashsum 🔍

**Multi-threaded file checksum calculator** | [![MIT License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

Calculate multiple file checksums simultaneously with blazing-fast performance using Rust's concurrency capabilities.


## ✨ Features

- ⚡ **Parallel computation** of 5 hash algorithms
- 📊 Supported algorithms:
  - CRC32 (IEEE 802.3)
  - MD5
  - SHA-1
  - SHA-256
  - SHA-512
- 📁 Large file support (>10GB tested)
- 💾 Save results to `.checksum` files
- ⏱️ Execution time metrics
- 📦 **Cross-platform** (Windows/Linux/macOS)

## 🚀 Installation

### Option 1: Linux (Portable)
Download the portable package from the [releases page](https://github.com/NDXDeveloper/slashsum/releases):
```bash
tar -xzvf slashsum-linux-amd64-portable.tar.gz
cd slashsum-linux-amd64-portable
sudo ./install.sh
```

### Option 2: Linux (Snap)
Download the `.snap` file from the [releases page](https://github.com/NDXDeveloper/slashsum/releases) and install:
```bash
sudo snap install slashsum_<version>_amd64.snap --dangerous --classic
```

### Option 3: Linux (DEB - Debian/Ubuntu)
Download the `.deb` file from the [releases page](https://github.com/NDXDeveloper/slashsum/releases) and install:
```bash
sudo dpkg -i slashsum_<version>_amd64.deb
```

### Option 4: Windows
Download the installer from the [releases page](https://github.com/NDXDeveloper/slashsum/releases):
- `slashsum-setup-user-<version>.exe` - Install for current user only (no admin required)
- `slashsum-setup-admin-<version>.exe` - System-wide installation (requires admin)

### Option 5: macOS
Download the portable package from the [releases page](https://github.com/NDXDeveloper/slashsum/releases):
```bash
tar -xzvf slashsum-darwin-amd64-portable.tar.gz
cd slashsum-darwin-amd64-portable
sudo ./install.sh
```

### Option 6: Build from source
```bash
git clone https://github.com/NDXDeveloper/slashsum
cd slashsum
cargo build --release
```
The binary will be available at `target/release/slashsum`.

## 🛠 Usage

Basic usage:
```bash
slashsum path/to/file [--save]
```

Example output:
```text
File:     large_file.iso
Size:     4.68 GB (5033165312 bytes)
CRC32:    8d7be4e9
MD5:      a3b9d148c5f8d237f735a5d9795a2345
SHA1:     2aae6c35c94fcfb415dbe95f408b9ce91ee846ed
SHA256:   b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
SHA512:   309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f
Time:     12.45s
```

## 📈 Performance Benchmarks

| File Size | Threads | Time (s) |
|-----------|---------|----------|
| 100 MB    | 5       | 0.45     |
| 1 GB      | 5       | 2.34     |
| 10 GB     | 5       | 25.12    |
| 50 GB     | 5       | 128.70   |

Tested on AWS EC2 t2.xlarge instance (4 vCPUs, 16GB RAM)

## 🧠 How It Works

1. **File Reading**  
   Buffered reading in 1MB chunks (configurable)
2. **Data Distribution**  
   Uses crossbeam channels to send chunks to hash workers
3. **Parallel Processing**  
   Dedicated thread for each hash algorithm
4. **Result Aggregation**  
   Combines results from all threads
5. **Output Formatting**  
   Human-readable sizes and standardized hash formats

## 🛠 Dependencies

- [crossbeam-channel](https://docs.rs/crossbeam-channel) - Thread communication
- [Rust Crypto Hashes](https://github.com/RustCrypto/hashes) - Cryptographic implementations
- [crc](https://docs.rs/crc) - CRC32 calculation

## 🤝 Contributing

Contributions are welcome!
See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 👤 Author

**Nicolas DEOUX**

- [NDXDev@gmail.com](mailto:NDXDev@gmail.com)
- [LinkedIn](https://www.linkedin.com/in/nicolas-deoux-ab295980/)
- [GitHub](https://github.com/NDXDeveloper)

## 📜 License

MIT License - See [LICENSE](LICENSE)

---

🔍 **Why "Slashsum"?**
Combination of "slash" (/) for file paths and "checksum" - because every good tool needs a catchy name!

