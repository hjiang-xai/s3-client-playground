# Changelog

## Recent Updates

### New Features

1. **Latency Tracking** ✅
   - All operations (PUT, GET, LIST) now report average latency in milliseconds
   - Helps identify performance bottlenecks
   - Shows per-operation timing metrics

2. **Range Query Support for GET** ✅
   - New `--range-bytes` option for GET operations
   - Read only the first N bytes of objects
   - Useful for testing metadata reads or index lookups
   - Example: `--range-bytes 100` reads first 100 bytes

3. **Updated Object Sizes** ✅
   - Quick test now uses 1GiB objects with 8MiB parts (matching warp config)
   - Configurable via command-line arguments

4. **Prefix Format** ✅
   - All prefixes now end with `/` (required by your S3 implementation)
   - Updated all examples and scripts

### Configuration

The tool now fully supports your warp configuration:
- Object size: 1 GiB (configurable with `--object-size`)
- Part size: 8 MiB (configurable with `--part-size`)
- Concurrent operations: 200 (configurable with `--concurrent`)
- Duration: 60 seconds (configurable with `--duration-secs`)

### Example Output

```
=== PUT Benchmark Results ===
Duration: 60.05s
Total operations: 1234
Successful: 1230
Errors: 4
Operations/sec: 20.48
Average latency: 245.67 ms          ← NEW!
Data transferred: 1230.00 MB
Throughput: 20.48 MB/s
```

### Usage Examples

#### PUT with 1GiB objects
```bash
cargo run --release -- put \
  --endpoint "http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz" \
  --bucket "hjiang-benchmark-o2-nov-19-toast63" \
  --duration-secs 60 \
  --concurrent 200 \
  --object-size 1073741824 \
  --part-size 8388608 \
  --prefix "loadtest/"
```

#### GET with range query (100 bytes)
```bash
cargo run --release -- get \
  --endpoint "http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz" \
  --bucket "hjiang-benchmark-o2-nov-19-toast63" \
  --duration-secs 60 \
  --concurrent 200 \
  --prefix "loadtest/" \
  --range-bytes 100     ← NEW!
```

#### LIST with latency tracking
```bash
cargo run --release -- list \
  --endpoint "http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz" \
  --bucket "hjiang-benchmark-o2-nov-19-toast63" \
  --duration-secs 60 \
  --concurrent 10 \
  --prefix "loadtest/"
```

### Scripts Updated

- `quick_test.sh`: Now uses 1GiB objects and 100-byte range queries
- `run_benchmark.sh`: Matches your warp config exactly with range queries

### What's Next

You can now:
1. Run `./quick_test.sh` to validate the setup (10 seconds, 1GiB objects)
2. Run `./run_benchmark.sh` for full benchmarks (60 seconds, 200 concurrent)
3. Customize any parameters via command-line arguments
4. Monitor latency metrics to identify performance issues

