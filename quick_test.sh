#!/bin/bash

# Quick test with smaller parameters for validation
# Use this before running the full benchmark

ENDPOINT="http://o2.pool-toast.service.capi-prod-storage.kube.atla.twitter.biz"
BUCKET="hjiang-benchmark-o2-nov-19-toast63"
ACCESS_KEY="changeme"
SECRET_KEY="changeme"
REGION="atla"

echo "Running quick S3 tests (10 seconds, 1GiB objects)..."
echo "======================================================="

# Quick PUT test with 1GiB objects and 8MiB parts
echo ""
echo "Testing PUT (1GiB objects, 8MiB parts, 10 concurrent, 10 seconds)..."
cargo run --release -- put \
  --endpoint "$ENDPOINT" \
  --bucket "$BUCKET" \
  --access-key "$ACCESS_KEY" \
  --secret-key "$SECRET_KEY" \
  --region "$REGION" \
  --duration-secs 10 \
  --concurrent 10 \
  --object-size 1073741824 \
  --part-size 8388608 \
  --prefix "quicktest/"

# Quick GET test with range query (read first 100 bytes)
echo ""
echo "Testing GET with range query (first 100 bytes, 10 concurrent, 10 seconds)..."
cargo run --release -- get \
  --endpoint "$ENDPOINT" \
  --bucket "$BUCKET" \
  --access-key "$ACCESS_KEY" \
  --secret-key "$SECRET_KEY" \
  --region "$REGION" \
  --duration-secs 10 \
  --concurrent 10 \
  --prefix "quicktest/" \
  --range-bytes 100

# Quick LIST test
echo ""
echo "Testing LIST (10 concurrent, 10 seconds)..."
cargo run --release -- list \
  --endpoint "$ENDPOINT" \
  --bucket "$BUCKET" \
  --access-key "$ACCESS_KEY" \
  --secret-key "$SECRET_KEY" \
  --region "$REGION" \
  --duration-secs 10 \
  --concurrent 10 \
  --prefix "quicktest/"

echo ""
echo "Quick test completed!"

