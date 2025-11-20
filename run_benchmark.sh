#!/bin/bash

# S3 Configuration (based on your warp config)
ENDPOINT="http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz"
BUCKET="hjiang-benchmark-o2-nov-19-toast63"
ACCESS_KEY="changeme"
SECRET_KEY="changeme"
REGION="atla"

# Benchmark Configuration
DURATION=60
CONCURRENT=200
OBJECT_SIZE=$((1 * 1024 * 1024 * 1024))  # 1 GiB
PART_SIZE=$((8 * 1024 * 1024))           # 8 MiB

echo "Running S3 Load Tests..."
echo "========================"

# Run PUT benchmark
echo ""
echo "1. Running PUT benchmark (1 GiB objects, 200 concurrent)..."
cargo run --release -- put \
  --endpoint "$ENDPOINT" \
  --bucket "$BUCKET" \
  --access-key "$ACCESS_KEY" \
  --secret-key "$SECRET_KEY" \
  --region "$REGION" \
  --duration-secs $DURATION \
  --concurrent $CONCURRENT \
  --object-size $OBJECT_SIZE \
  --part-size $PART_SIZE \
  --prefix "loadtest/"

# Run GET benchmark with range query
echo ""
echo "2. Running GET benchmark with range query (first 100 bytes, 200 concurrent)..."
cargo run --release -- get \
  --endpoint "$ENDPOINT" \
  --bucket "$BUCKET" \
  --access-key "$ACCESS_KEY" \
  --secret-key "$SECRET_KEY" \
  --region "$REGION" \
  --duration-secs $DURATION \
  --concurrent $CONCURRENT \
  --prefix "loadtest/" \
  --range-bytes 100

# Run LIST benchmark
echo ""
echo "3. Running LIST benchmark (10 concurrent)..."
cargo run --release -- list \
  --endpoint "$ENDPOINT" \
  --bucket "$BUCKET" \
  --access-key "$ACCESS_KEY" \
  --secret-key "$SECRET_KEY" \
  --region "$REGION" \
  --duration-secs $DURATION \
  --concurrent 10 \
  --prefix "loadtest/"

echo ""
echo "All benchmarks completed!"

