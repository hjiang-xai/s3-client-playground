use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use aws_sdk_s3::Client as S3Client;
use bytes::Bytes;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use rand::RngCore;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

#[derive(Parser)]
#[command(name = "s3-load-gen")]
#[command(about = "S3 Load Testing Tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run PUT benchmark
    Put {
        #[arg(long, default_value = "changeme")]
        access_key: String,
        #[arg(long, default_value = "changeme")]
        secret_key: String,
        #[arg(long, default_value = "us-east-1")]
        region: String,
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        bucket: String,
        #[arg(long, default_value = "60")]
        duration_secs: u64,
        #[arg(long, default_value = "10")]
        concurrent: usize,
        #[arg(long, default_value = "1048576")] // 1MB default
        object_size: usize,
        #[arg(long, default_value = "8388608")] // 8MB default
        part_size: usize,
        #[arg(long)]
        disable_multipart: bool,
        #[arg(long, default_value = "test-object/")]
        prefix: String,
    },
    /// Run GET benchmark
    Get {
        #[arg(long, default_value = "changeme")]
        access_key: String,
        #[arg(long, default_value = "changeme")]
        secret_key: String,
        #[arg(long, default_value = "us-east-1")]
        region: String,
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        bucket: String,
        #[arg(long, default_value = "60")]
        duration_secs: u64,
        #[arg(long, default_value = "10")]
        concurrent: usize,
        #[arg(long, default_value = "test-object/")]
        prefix: String,
        #[arg(long)]
        range_bytes: Option<usize>,
    },
    /// Run LIST benchmark
    List {
        #[arg(long, default_value = "changeme")]
        access_key: String,
        #[arg(long, default_value = "changeme")]
        secret_key: String,
        #[arg(long, default_value = "us-east-1")]
        region: String,
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        bucket: String,
        #[arg(long, default_value = "60")]
        duration_secs: u64,
        #[arg(long, default_value = "10")]
        concurrent: usize,
        #[arg(long, default_value = "")]
        prefix: String,
    },
}

struct Stats {
    operations: u64,
    bytes_transferred: u64,
    errors: u64,
    duration: Duration,
    total_latency_ms: f64,
}

impl Stats {
    fn print(&self, operation: &str) {
        let ops_per_sec = self.operations as f64 / self.duration.as_secs_f64();
        let mb_per_sec = (self.bytes_transferred as f64 / 1_048_576.0) / self.duration.as_secs_f64();
        let successful = self.operations - self.errors;
        let avg_latency_ms = if successful > 0 {
            self.total_latency_ms / successful as f64
        } else {
            0.0
        };
        
        println!("\n=== {} Benchmark Results ===", operation);
        println!("Duration: {:.2}s", self.duration.as_secs_f64());
        println!("Total operations: {}", self.operations);
        println!("Successful: {}", successful);
        println!("Errors: {}", self.errors);
        println!("Operations/sec: {:.2}", ops_per_sec);
        println!("Average latency: {:.2} ms", avg_latency_ms);
        println!("Data transferred: {:.2} MB", self.bytes_transferred as f64 / 1_048_576.0);
        println!("Throughput: {:.2} MB/s", mb_per_sec);
    }
}

fn create_s3_client(access_key: String, secret_key: String, region: String, endpoint: String) -> S3Client {
    let credentials = Credentials::new(access_key, secret_key, None, None, "static");
    
    let config = S3ConfigBuilder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(region))
        .endpoint_url(endpoint)
        .credentials_provider(credentials)
        .force_path_style(true)
        .build();
    
    S3Client::from_conf(config)
}

fn generate_random_data(size: usize) -> Vec<u8> {
    let mut data = vec![0u8; size];
    rand::thread_rng().fill_bytes(&mut data);
    data
}

async fn put_object_simple(
    client: &S3Client,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
) -> Result<usize> {
    let size = data.len();
    println!("[PUT] Starting simple upload for key: {} (size: {} bytes)", key, size);
    let body = ByteStream::from(data);
    
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send()
        .await
        .context("Failed to put object")?;
    
    println!("[PUT] Completed simple upload for key: {}", key);
    Ok(size)
}

async fn put_object_multipart(
    client: &S3Client,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
    part_size: usize,
) -> Result<usize> {
    let total_size = data.len();
    let num_parts = (total_size + part_size - 1) / part_size;
    
    println!("[PUT-MP] Starting multipart upload for key: {} (size: {} bytes, {} parts)", key, total_size, num_parts);
    
    // Initiate multipart upload
    let multipart = client
        .create_multipart_upload()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .context("Failed to create multipart upload")?;
    
    let upload_id = multipart.upload_id().context("No upload ID")?;
    println!("[PUT-MP] Created upload ID: {} for key: {}", upload_id, key);
    
    // Upload parts in parallel
    let mut upload_tasks = Vec::new();
    let mut part_number = 1;
    
    for chunk in data.chunks(part_size) {
        let client = client.clone();
        let bucket = bucket.to_string();
        let key = key.to_string();
        let upload_id = upload_id.to_string();
        let chunk_data = Bytes::copy_from_slice(chunk);
        let current_part = part_number;
        
        println!("[PUT-MP] Spawning upload task for part {} of {} for key: {}", current_part, num_parts, key);
        
        let task = tokio::spawn(async move {
            println!("[PUT-MP] Uploading part {} for key: {}", current_part, key);
            let body = ByteStream::from(chunk_data);
            
            let result = client
                .upload_part()
                .bucket(bucket)
                .key(&key)
                .upload_id(upload_id)
                .part_number(current_part)
                .body(body)
                .send()
                .await;
            
            match &result {
                Ok(_) => println!("[PUT-MP] Completed part {} for key: {}", current_part, key),
                Err(e) => println!("[PUT-MP] Failed part {} for key: {} - {:?}", current_part, key, e),
            }
            
            result.map(|resp| (current_part, resp))
        });
        
        upload_tasks.push(task);
        part_number += 1;
    }
    
    println!("[PUT-MP] Waiting for {} parallel part uploads to complete for key: {}", upload_tasks.len(), key);
    
    // Collect results from all parallel uploads
    let mut completed_parts = Vec::new();
    for task in upload_tasks {
        let (part_num, upload_result) = task
            .await
            .context("Upload part task panicked")?
            .context("Failed to upload part")?;
        
        completed_parts.push(
            CompletedPart::builder()
                .part_number(part_num)
                .e_tag(upload_result.e_tag().unwrap_or_default())
                .build(),
        );
    }
    
    // Sort parts by part number (important for S3)
    completed_parts.sort_by_key(|p| p.part_number());
    
    // Complete multipart upload
    println!("[PUT-MP] Completing multipart upload for key: {}", key);
    let completed_upload = CompletedMultipartUpload::builder()
        .set_parts(Some(completed_parts))
        .build();
    
    client
        .complete_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .multipart_upload(completed_upload)
        .send()
        .await
        .context("Failed to complete multipart upload")?;
    
    println!("[PUT-MP] Successfully completed multipart upload for key: {}", key);
    Ok(total_size)
}

async fn get_object(client: &S3Client, bucket: &str, key: &str) -> Result<usize> {
    println!("[GET] Starting download for key: {}", key);
    let resp = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .context("Failed to get object")?;
    
    let data = resp.body.collect().await.context("Failed to read body")?;
    let size = data.into_bytes().len();
    println!("[GET] Completed download for key: {} (size: {} bytes)", key, size);
    Ok(size)
}

async fn get_object_range(client: &S3Client, bucket: &str, key: &str, range_bytes: usize) -> Result<usize> {
    println!("[GET-RANGE] Starting range download for key: {} (first {} bytes)", key, range_bytes);
    let range = format!("bytes=0-{}", range_bytes - 1);
    let resp = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .range(range)
        .send()
        .await
        .context("Failed to get object range")?;
    
    let data = resp.body.collect().await.context("Failed to read body")?;
    let size = data.into_bytes().len();
    println!("[GET-RANGE] Completed range download for key: {} (size: {} bytes)", key, size);
    Ok(size)
}

async fn list_objects(client: &S3Client, bucket: &str, prefix: &str) -> Result<usize> {
    println!("[LIST] Starting list operation with prefix: '{}'", prefix);
    let mut count = 0;
    let mut continuation_token: Option<String> = None;
    let mut page = 1;
    
    loop {
        println!("[LIST] Fetching page {} for prefix: '{}'", page, prefix);
        let mut request = client.list_objects_v2().bucket(bucket).max_keys(1000);
        
        if !prefix.is_empty() {
            request = request.prefix(prefix);
        }
        
        if let Some(token) = continuation_token {
            request = request.continuation_token(token);
        }
        
        let resp = request.send().await.context("Failed to list objects")?;
        
        let page_count = resp.contents().len();
        count += page_count;
        println!("[LIST] Page {} returned {} objects (total so far: {})", page, page_count, count);
        
        if resp.is_truncated() == Some(true) {
            continuation_token = resp.next_continuation_token().map(String::from);
            page += 1;
        } else {
            break;
        }
    }
    
    println!("[LIST] Completed list operation with prefix: '{}' (total: {} objects)", prefix, count);
    Ok(count)
}

async fn run_put_benchmark(
    access_key: String,
    secret_key: String,
    region: String,
    endpoint: String,
    bucket: String,
    duration_secs: u64,
    concurrent: usize,
    object_size: usize,
    part_size: usize,
    disable_multipart: bool,
    prefix: String,
) -> Result<()> {
    let client = Arc::new(create_s3_client(access_key, secret_key, region, endpoint.clone()));
    let semaphore = Arc::new(Semaphore::new(concurrent));
    let duration = Duration::from_secs(duration_secs);
    
    println!("Starting PUT benchmark...");
    println!("Endpoint: {}", endpoint);
    println!("Bucket: {}", bucket);
    println!("Duration: {}s", duration_secs);
    println!("Concurrent operations: {}", concurrent);
    println!("Object size: {} bytes ({:.2} MB)", object_size, object_size as f64 / 1_048_576.0);
    println!("Part size: {} bytes ({:.2} MB)", part_size, part_size as f64 / 1_048_576.0);
    println!("Multipart: {}", !disable_multipart);
    
    let start = Instant::now();
    let mut tasks = Vec::new();
    let mut operation_count = 0u64;
    let mut bytes_transferred = 0u64;
    let mut errors = 0u64;
    let mut total_latency_ms = 0.0;
    
    let pb = ProgressBar::new(duration_secs);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len}s ({msg})")
        .unwrap()
        .progress_chars("#>-"));
    
    while start.elapsed() < duration {
        let permit = semaphore.clone().acquire_owned().await?;
        let client = client.clone();
        let bucket = bucket.clone();
        let key = format!("{}{}-{}", prefix, operation_count, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        
        println!("[BENCH] Generating random data for operation {} (size: {} bytes)", operation_count, object_size);
        let data = generate_random_data(object_size);
        
        println!("[BENCH] Spawning PUT task {} for key: {}", operation_count, key);
        let task = tokio::spawn(async move {
            let op_start = Instant::now();
            let result = if disable_multipart || object_size < part_size {
                put_object_simple(&client, &bucket, &key, data).await
            } else {
                put_object_multipart(&client, &bucket, &key, data, part_size).await
            };
            let latency = op_start.elapsed();
            drop(permit);
            (result, latency)
        });
        
        tasks.push(task);
        operation_count += 1;
        
        pb.set_message(format!("ops: {}, errors: {}", operation_count, errors));
        pb.set_position(start.elapsed().as_secs().min(duration_secs));
        
        // Small delay to prevent overwhelming the system
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    println!("[BENCH] Duration reached, waiting for {} in-flight operations to complete...", tasks.len());
    pb.finish_with_message("Waiting for all operations to complete...");
    
    // Wait for all tasks to complete
    println!("[BENCH] Collecting results from {} tasks...", tasks.len());
    for (idx, task) in tasks.into_iter().enumerate() {
        println!("[BENCH] Waiting for task {} of {} to complete...", idx + 1, operation_count);
        match task.await {
            Ok((Ok(size), latency)) => {
                println!("[BENCH] Task {} succeeded: {} bytes in {:.2}ms", idx + 1, size, latency.as_secs_f64() * 1000.0);
                bytes_transferred += size as u64;
                total_latency_ms += latency.as_secs_f64() * 1000.0;
            }
            Ok((Err(e), _)) => {
                println!("[BENCH] Task {} failed with error: {:?}", idx + 1, e);
                errors += 1;
            }
            Err(e) => {
                println!("[BENCH] Task {} panicked: {:?}", idx + 1, e);
                errors += 1;
            }
        }
    }
    
    println!("[BENCH] All PUT tasks completed!");
    
    let total_duration = start.elapsed();
    
    let stats = Stats {
        operations: operation_count,
        bytes_transferred,
        errors,
        duration: total_duration,
        total_latency_ms,
    };
    
    stats.print("PUT");
    
    Ok(())
}

async fn run_get_benchmark(
    access_key: String,
    secret_key: String,
    region: String,
    endpoint: String,
    bucket: String,
    duration_secs: u64,
    concurrent: usize,
    prefix: String,
    range_bytes: Option<usize>,
) -> Result<()> {
    let client = Arc::new(create_s3_client(access_key, secret_key, region, endpoint.clone()));
    let semaphore = Arc::new(Semaphore::new(concurrent));
    let duration = Duration::from_secs(duration_secs);
    
    println!("Starting GET benchmark...");
    println!("Endpoint: {}", endpoint);
    println!("Bucket: {}", bucket);
    println!("Duration: {}s", duration_secs);
    println!("Concurrent operations: {}", concurrent);
    if let Some(bytes) = range_bytes {
        println!("Range query: reading first {} bytes", bytes);
    }
    
    // First, list objects to know what to get
    println!("Listing objects with prefix '{}'...", prefix);
    let mut objects = Vec::new();
    let mut continuation_token: Option<String> = None;
    
    loop {
        let mut request = client.list_objects_v2().bucket(&bucket).max_keys(1000);
        
        if !prefix.is_empty() {
            request = request.prefix(&prefix);
        }
        
        if let Some(token) = continuation_token {
            request = request.continuation_token(token);
        }
        
        let resp = request.send().await.context("Failed to list objects")?;
        
        for obj in resp.contents() {
            if let Some(key) = obj.key() {
                objects.push(key.to_string());
            }
        }
        
        if resp.is_truncated() == Some(true) {
            continuation_token = resp.next_continuation_token().map(String::from);
        } else {
            break;
        }
    }
    
    if objects.is_empty() {
        anyhow::bail!("No objects found with prefix '{}'. Please run PUT benchmark first.", prefix);
    }
    
    println!("Found {} objects to download", objects.len());
    
    let start = Instant::now();
    let mut tasks = Vec::new();
    let mut operation_count = 0u64;
    let mut bytes_transferred = 0u64;
    let mut errors = 0u64;
    let mut total_latency_ms = 0.0;
    let mut object_index = 0;
    
    let pb = ProgressBar::new(duration_secs);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len}s ({msg})")
        .unwrap()
        .progress_chars("#>-"));
    
    while start.elapsed() < duration {
        let permit = semaphore.clone().acquire_owned().await?;
        let client = client.clone();
        let bucket = bucket.clone();
        let key = objects[object_index % objects.len()].clone();
        object_index += 1;
        
        println!("[BENCH] Spawning GET task {} for key: {}", operation_count, key);
        let task = tokio::spawn(async move {
            let op_start = Instant::now();
            let result = if let Some(bytes) = range_bytes {
                get_object_range(&client, &bucket, &key, bytes).await
            } else {
                get_object(&client, &bucket, &key).await
            };
            let latency = op_start.elapsed();
            drop(permit);
            (result, latency)
        });
        
        tasks.push(task);
        operation_count += 1;
        
        pb.set_message(format!("ops: {}, errors: {}", operation_count, errors));
        pb.set_position(start.elapsed().as_secs().min(duration_secs));
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    println!("[BENCH] Duration reached, waiting for {} in-flight GET operations to complete...", tasks.len());
    pb.finish_with_message("Waiting for all operations to complete...");
    
    // Wait for all tasks to complete
    println!("[BENCH] Collecting results from {} GET tasks...", tasks.len());
    for (idx, task) in tasks.into_iter().enumerate() {
        println!("[BENCH] Waiting for GET task {} of {} to complete...", idx + 1, operation_count);
        match task.await {
            Ok((Ok(size), latency)) => {
                println!("[BENCH] GET task {} succeeded: {} bytes in {:.2}ms", idx + 1, size, latency.as_secs_f64() * 1000.0);
                bytes_transferred += size as u64;
                total_latency_ms += latency.as_secs_f64() * 1000.0;
            }
            Ok((Err(e), _)) => {
                println!("[BENCH] GET task {} failed with error: {:?}", idx + 1, e);
                errors += 1;
            }
            Err(e) => {
                println!("[BENCH] GET task {} panicked: {:?}", idx + 1, e);
                errors += 1;
            }
        }
    }
    
    println!("[BENCH] All GET tasks completed!");
    
    let total_duration = start.elapsed();
    
    let stats = Stats {
        operations: operation_count,
        bytes_transferred,
        errors,
        duration: total_duration,
        total_latency_ms,
    };
    
    stats.print("GET");
    
    Ok(())
}

async fn run_list_benchmark(
    access_key: String,
    secret_key: String,
    region: String,
    endpoint: String,
    bucket: String,
    duration_secs: u64,
    concurrent: usize,
    prefix: String,
) -> Result<()> {
    let client = Arc::new(create_s3_client(access_key, secret_key, region, endpoint.clone()));
    let semaphore = Arc::new(Semaphore::new(concurrent));
    let duration = Duration::from_secs(duration_secs);
    
    println!("Starting LIST benchmark...");
    println!("Endpoint: {}", endpoint);
    println!("Bucket: {}", bucket);
    println!("Duration: {}s", duration_secs);
    println!("Concurrent operations: {}", concurrent);
    println!("Prefix: '{}'", prefix);
    
    let start = Instant::now();
    let mut tasks = Vec::new();
    let mut operation_count = 0u64;
    let mut errors = 0u64;
    let mut total_objects_listed = 0u64;
    let mut total_latency_ms = 0.0;
    
    let pb = ProgressBar::new(duration_secs);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len}s ({msg})")
        .unwrap()
        .progress_chars("#>-"));
    
    while start.elapsed() < duration {
        let permit = semaphore.clone().acquire_owned().await?;
        let client = client.clone();
        let bucket = bucket.clone();
        let prefix = prefix.clone();
        
        println!("[BENCH] Spawning LIST task {} with prefix: '{}'", operation_count, prefix);
        let task = tokio::spawn(async move {
            let op_start = Instant::now();
            let result = list_objects(&client, &bucket, &prefix).await;
            let latency = op_start.elapsed();
            drop(permit);
            (result, latency)
        });
        
        tasks.push(task);
        operation_count += 1;
        
        pb.set_message(format!("ops: {}, errors: {}", operation_count, errors));
        pb.set_position(start.elapsed().as_secs().min(duration_secs));
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    println!("[BENCH] Duration reached, waiting for {} in-flight LIST operations to complete...", tasks.len());
    pb.finish_with_message("Waiting for all operations to complete...");
    
    // Wait for all tasks to complete
    println!("[BENCH] Collecting results from {} LIST tasks...", tasks.len());
    for (idx, task) in tasks.into_iter().enumerate() {
        println!("[BENCH] Waiting for LIST task {} of {} to complete...", idx + 1, operation_count);
        match task.await {
            Ok((Ok(count), latency)) => {
                println!("[BENCH] LIST task {} succeeded: {} objects in {:.2}ms", idx + 1, count, latency.as_secs_f64() * 1000.0);
                total_objects_listed += count as u64;
                total_latency_ms += latency.as_secs_f64() * 1000.0;
            }
            Ok((Err(e), _)) => {
                println!("[BENCH] LIST task {} failed with error: {:?}", idx + 1, e);
                errors += 1;
            }
            Err(e) => {
                println!("[BENCH] LIST task {} panicked: {:?}", idx + 1, e);
                errors += 1;
            }
        }
    }
    
    println!("[BENCH] All LIST tasks completed!");
    
    let total_duration = start.elapsed();
    
    let stats = Stats {
        operations: operation_count,
        bytes_transferred: 0,
        errors,
        duration: total_duration,
        total_latency_ms,
    };
    
    stats.print("LIST");
    println!("Total objects listed: {}", total_objects_listed);
    println!("Avg objects per list: {:.2}", total_objects_listed as f64 / operation_count as f64);
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Put {
            access_key,
            secret_key,
            region,
            endpoint,
            bucket,
            duration_secs,
            concurrent,
            object_size,
            part_size,
            disable_multipart,
            prefix,
        } => {
            run_put_benchmark(
                access_key,
                secret_key,
                region,
                endpoint,
                bucket,
                duration_secs,
                concurrent,
                object_size,
                part_size,
                disable_multipart,
                prefix,
            )
            .await?;
        }
        Commands::Get {
            access_key,
            secret_key,
            region,
            endpoint,
            bucket,
            duration_secs,
            concurrent,
            prefix,
            range_bytes,
        } => {
            run_get_benchmark(
                access_key,
                secret_key,
                region,
                endpoint,
                bucket,
                duration_secs,
                concurrent,
                prefix,
                range_bytes,
            )
            .await?;
        }
        Commands::List {
            access_key,
            secret_key,
            region,
            endpoint,
            bucket,
            duration_secs,
            concurrent,
            prefix,
        } => {
            run_list_benchmark(
                access_key,
                secret_key,
                region,
                endpoint,
                bucket,
                duration_secs,
                concurrent,
                prefix,
            )
            .await?;
        }
    }
    
    Ok(())
}

