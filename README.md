# S3 Load Generator

A Rust-based S3 load testing tool that supports PUT, GET, and LIST operations with configurable concurrency and object sizes.

## Features

- **PUT Benchmark**: Upload objects with configurable size, supports multipart uploads
- **GET Benchmark**: Download objects with concurrent requests, supports range queries
- **LIST Benchmark**: List objects with configurable prefix
- Configurable concurrency levels
- Real-time progress tracking
- Detailed performance statistics including average latency per operation

## Installation

```bash
cargo build --release
```

## Usage

### PUT Benchmark

Upload objects to S3 cluster:

```bash
cargo run --release -- put \
  --endpoint "http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz" \
  --bucket "hjiang-benchmark-o2-nov-19-toast63" \
  --access-key "changeme" \
  --secret-key "changeme" \
  --region "atla" \
  --duration-secs 60 \
  --concurrent 200 \
  --object-size 1073741824 \
  --part-size 8388608 \
  --prefix "loadtest/"
```

Options:
- `--endpoint`: S3 endpoint URL
- `--bucket`: Target bucket name
- `--access-key`: S3 access key
- `--secret-key`: S3 secret key
- `--region`: AWS region (default: "us-east-1")
- `--duration-secs`: Benchmark duration in seconds (default: 60)
- `--concurrent`: Number of concurrent operations (default: 10)
- `--object-size`: Size of each object in bytes (default: 1048576 = 1MB)
- `--part-size`: Multipart upload part size in bytes (default: 8388608 = 8MB)
- `--disable-multipart`: Disable multipart uploads
- `--prefix`: Object key prefix (default: "test-object/") - Note: must end with `/` for this S3 implementation

### GET Benchmark

Download objects from S3 cluster:

```bash
cargo run --release -- get \
  --endpoint "http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz" \
  --bucket "hjiang-benchmark-o2-nov-19-toast63" \
  --access-key "changeme" \
  --secret-key "changeme" \
  --region "atla" \
  --duration-secs 60 \
  --concurrent 200 \
  --prefix "loadtest/"
```

Options:
- `--endpoint`: S3 endpoint URL
- `--bucket`: Target bucket name
- `--access-key`: S3 access key
- `--secret-key`: S3 secret key
- `--region`: AWS region
- `--duration-secs`: Benchmark duration in seconds
- `--concurrent`: Number of concurrent operations
- `--prefix`: Object key prefix to filter downloads
- `--range-bytes`: Optional - Read only first N bytes (range query)

### LIST Benchmark

List objects in S3 bucket:

```bash
cargo run --release -- list \
  --endpoint "http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz" \
  --bucket "hjiang-benchmark-o2-nov-19-toast63" \
  --access-key "changeme" \
  --secret-key "changeme" \
  --region "atla" \
  --duration-secs 60 \
  --concurrent 10 \
  --prefix ""
```

Options:
- `--endpoint`: S3 endpoint URL
- `--bucket`: Target bucket name
- `--access-key`: S3 access key
- `--secret-key`: S3 secret key
- `--region`: AWS region
- `--duration-secs`: Benchmark duration in seconds
- `--concurrent`: Number of concurrent operations
- `--prefix`: Object key prefix to filter listings (default: empty = list all)

## Size Units Reference

Common object sizes in bytes:
- 1 KB = 1024
- 1 MB = 1048576
- 8 MB = 8388608
- 100 MB = 104857600
- 1 GB = 1073741824

## Example Output

```
=== PUT Benchmark Results ===
Duration: 60.05s
Total operations: 1234
Successful: 1230
Errors: 4
Operations/sec: 20.48
Average latency: 245.67 ms
Data transferred: 1230.00 MB
Throughput: 20.48 MB/s
```

## Performance Tips

1. For large objects (>100MB), enable multipart uploads with appropriate part size
2. Adjust concurrency based on your network and system capabilities
3. Use SSD storage for better local performance
4. Monitor network bandwidth during testing
5. For maximum throughput, use `--release` build mode

## Requirements

- Rust 1.70 or later
- Network access to S3 endpoint
- Sufficient disk space for object generation (PUT) and storage (GET)

