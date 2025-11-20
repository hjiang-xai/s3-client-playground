# Usage Examples

## Quick Start

### Build the project
```bash
cargo build --release
```

### Run a quick test (1GiB objects, 8MiB parts)
```bash
./quick_test.sh
```

**Note:** This S3 implementation requires prefixes to end with `/`

### Run the full benchmark suite (matching your warp config)
```bash
./run_benchmark.sh
```

## Individual Commands

### PUT Examples

#### Small objects (1 MB, good for testing)
```bash
cargo run --release -- put \
  --endpoint "http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz" \
  --bucket "hjiang-benchmark-o2-nov-19-toast63" \
  --access-key "changeme" \
  --secret-key "changeme" \
  --region "atla" \
  --duration-secs 30 \
  --concurrent 50 \
  --object-size 1048576 \
  --part-size 524288 \
  --prefix "test/"
```

#### Medium objects (100 MB with multipart)
```bash
cargo run --release -- put \
  --endpoint "http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz" \
  --bucket "hjiang-benchmark-o2-nov-19-toast63" \
  --access-key "changeme" \
  --secret-key "changeme" \
  --region "atla" \
  --duration-secs 60 \
  --concurrent 100 \
  --object-size 104857600 \
  --part-size 8388608 \
  --prefix "medium/"
```

#### Large objects (1 GB - matching your warp config)
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
  --prefix "large-"
```

### GET Examples

#### High concurrency GET (full object download)
```bash
cargo run --release -- get \
  --endpoint "http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz" \
  --bucket "hjiang-benchmark-o2-nov-19-toast63" \
  --access-key "changeme" \
  --secret-key "changeme" \
  --region "atla" \
  --duration-secs 60 \
  --concurrent 200 \
  --prefix "large/"
```

#### Range query GET (first 100 bytes only)
```bash
cargo run --release -- get \
  --endpoint "http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz" \
  --bucket "hjiang-benchmark-o2-nov-19-toast63" \
  --access-key "changeme" \
  --secret-key "changeme" \
  --region "atla" \
  --duration-secs 60 \
  --concurrent 200 \
  --prefix "large/" \
  --range-bytes 100
```

#### Range query GET (first 1KB)
```bash
cargo run --release -- get \
  --endpoint "http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz" \
  --bucket "hjiang-benchmark-o2-nov-19-toast63" \
  --access-key "changeme" \
  --secret-key "changeme" \
  --region "atla" \
  --duration-secs 30 \
  --concurrent 50 \
  --prefix "test/" \
  --range-bytes 1024
```

### LIST Examples

#### List all objects
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

#### List with prefix filter
```bash
cargo run --release -- list \
  --endpoint "http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz" \
  --bucket "hjiang-benchmark-o2-nov-19-toast63" \
  --access-key "changeme" \
  --secret-key "changeme" \
  --region "atla" \
  --duration-secs 30 \
  --concurrent 5 \
  --prefix "large/"
```

## Common Object Sizes

| Size | Bytes | Use Case |
|------|-------|----------|
| 1 KB | 1024 | Metadata/small files |
| 1 MB | 1048576 | Small objects, quick tests |
| 8 MB | 8388608 | Multipart chunk size |
| 10 MB | 10485760 | Medium objects |
| 100 MB | 104857600 | Large objects |
| 1 GB | 1073741824 | Very large objects (like warp config) |
| 5 GB | 5368709120 | Extra large objects |

## Performance Tuning Tips

1. **Concurrency**: Start with 10-50, increase gradually to find optimal throughput
2. **Object Size**: Larger objects test throughput, smaller objects test IOPS
3. **Part Size**: For multipart uploads, 5-8 MB is typically optimal
4. **Duration**: 60 seconds is usually enough to get stable metrics
5. **Prefix**: Use different prefixes for different test runs to keep data organized
6. **Range Queries**: Use `--range-bytes` for GET to test metadata/index read patterns without downloading full objects

## Monitoring Tips

During the benchmark, watch for:
- Operations per second (steady is good)
- Average latency (lower is better)
- Error rate (should be low)
- Network utilization
- CPU usage (should not be bottleneck)
- Throughput (MB/s)

## Latency Metrics

The tool now reports average latency for each operation type:
- **PUT latency**: Time from request start to complete upload (includes multipart overhead)
- **GET latency**: Time from request start to complete download (or range read)
- **LIST latency**: Time to complete one full listing operation (including pagination)

## Cleaning Up

After testing, you can delete test objects using AWS CLI or your preferred S3 client:

```bash
# Example with AWS CLI (if you have it installed)
aws s3 rm s3://hjiang-benchmark-o2-nov-19-toast63/loadtest/ --recursive \
  --endpoint-url http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz
```

