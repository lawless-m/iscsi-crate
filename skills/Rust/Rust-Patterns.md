---
name: Rust
description: Rust development patterns for video processing, trait abstractions, error handling, and Linux device interaction
---

# Rust Development Patterns

## Instructions

When helping users write Rust code, follow these guidelines:

1. **Trait Abstractions**: Use traits to define interfaces for swappable implementations (capture sources, output sinks, processing effects)

2. **Error Handling**:
   - Use `anyhow` for application/binary code (main, CLI, orchestration)
   - Use `thiserror` for library code that might be reused
   - Always use `Result` for fallible operations
   - Add context with `.context()` for better error messages

3. **CLI Pattern**: Use `clap` with derive macros for type-safe CLI argument parsing

4. **Logging**: Use `tracing` for structured logging with spans for performance measurement

5. **Module Organization**: Group related functionality in modules with clear public APIs via `mod.rs`

6. **Performance**: Measure frame timing, avoid allocations in hot loops, reuse buffers

7. **Linux Device I/O**: Direct file operations for v4l2 devices, proper color space conversion

8. **Edition**: Always use Rust 2021 edition

## Examples

### Example 1: Trait-Based Abstraction for Swappable Implementations

```
User: I need to support multiple camera backends (v4l2, nokhwa, test fixtures)

Claude: I'll create a trait abstraction that lets you swap implementations:

1. Define a CaptureSource trait with the core operations
2. Implement it for each backend (nokhwa, v4l, mock)
3. Use trait bounds in your pipeline function
4. This allows testing without real hardware

[Provides trait definition and multiple implementations from reference below]
```

### Example 2: Application vs Library Error Handling

```
User: Should I use anyhow or thiserror for my errors?

Claude: It depends on the context:

**Use anyhow when:**
- Writing application/binary code (main.rs, CLI)
- You want easy error context chaining
- Generic "something went wrong" is acceptable

**Use thiserror when:**
- Writing library code that others might use
- You need specific error types
- Callers need to match on error variants

[Provides examples of both patterns from reference below]
```

### Example 3: Frame Timing and Performance Monitoring

```
User: How do I measure frame processing performance?

Claude: I'll add timing instrumentation to your frame loop:

1. Use Instant::now() before/after each stage
2. Accumulate timings and log averages periodically
3. Calculate actual FPS from total processing time
4. Add tracing spans for detailed profiling

[Provides frame timing implementation from reference below]
```

### Example 4: v4l2 Device Output with Color Space Conversion

```
User: I need to output frames to a v4l2loopback virtual camera

Claude: I'll implement direct file writing with proper color conversion:

1. Open the v4l2 device file for writing
2. Convert RGB to YUYV (v4l2loopback's expected format)
3. Write raw frame data directly to the device
4. Handle frame resizing if needed

[Provides V4L2Output implementation from reference below]
```

---

# Reference Implementation Details

The sections below contain proven working code from the webcam-fx project.

## Project Structure

```
project/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI, orchestration
│   ├── lib.rs               # (optional) Re-exports for library usage
│   ├── module_name/
│   │   ├── mod.rs           # Module public API, trait definitions
│   │   └── impl_name.rs     # Specific implementations
│   └── pipeline/
│       └── runner.rs        # Main processing loop
├── models/                   # Model files (git-ignored)
└── assets/                   # Static resources
```

## Cargo.toml Conventions

```toml
[package]
name = "webcam-fx"
version = "0.1.0"
edition = "2021"

[dependencies]
# Error handling
anyhow = "1.0"           # For application code
thiserror = "1.0"        # For library code

# CLI
clap = { version = "4.0", features = ["derive"] }

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Image processing
image = "0.25"

# Domain-specific
nokhwa = { version = "0.10", features = ["input-v4l"] }
v4l = "0.14"
```

## Trait-Based Abstractions

### Define Interface Traits

```rust
// src/capture/mod.rs
use anyhow::Result;
use image::RgbImage;

/// Trait for camera capture sources
/// Allows swapping between real cameras, test fixtures, file playback
pub trait CaptureSource {
    /// Capture a single frame
    fn capture_frame(&mut self) -> Result<RgbImage>;

    /// Get the resolution of captured frames
    fn resolution(&self) -> (u32, u32);
}
```

### Implement for Specific Backends

```rust
// src/capture/nokhwa_impl.rs
use super::CaptureSource;
use anyhow::{Context, Result};
use image::RgbImage;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;

pub struct NokhwaCapture {
    camera: Camera,
    width: u32,
    height: u32,
}

impl NokhwaCapture {
    pub fn new(device_index: u32, width: u32, height: u32) -> Result<Self> {
        tracing::info!("Initializing camera {} at {}x{}", device_index, width, height);

        let index = CameraIndex::Index(device_index);
        let requested = RequestedFormat::new::<RgbFormat>(
            RequestedFormatType::AbsoluteHighestResolution
        );

        let mut camera = Camera::new(index, requested)
            .context("Failed to open camera")?;

        camera.open_stream()
            .context("Failed to open camera stream")?;

        Ok(Self { camera, width, height })
    }
}

impl CaptureSource for NokhwaCapture {
    fn capture_frame(&mut self) -> Result<RgbImage> {
        let frame = self.camera.frame()
            .context("Failed to capture frame")?;

        let decoded = frame.decode_image::<RgbFormat>()
            .context("Failed to decode frame")?;

        Ok(decoded)
    }

    fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
```

### Use with Trait Bounds

```rust
// src/main.rs or src/pipeline/runner.rs
fn run_pipeline<C, O>(capture: &mut C, output: &mut O, target_fps: u32) -> Result<()>
where
    C: CaptureSource,
    O: OutputSink,
{
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps as f32);

    loop {
        let frame = capture.capture_frame()?;
        output.write_frame(&frame)?;

        // Frame rate limiting
        std::thread::sleep(frame_duration);
    }
}
```

**Benefits:**
- Swap implementations without touching pipeline code
- Easy mocking for tests
- Clear interface contracts
- Type-safe at compile time

## Error Handling Patterns

### Application Code (anyhow)

```rust
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let args = Args::parse();

    // Easy error context chaining
    let capture = WebcamCapture::new(args.input_device, args.width, args.height)
        .context("Failed to initialize webcam capture")?;

    let output = V4L2Output::new(&args.output_device, args.output_width, args.output_height)
        .context("Failed to initialize v4l2loopback output")?;

    run_pipeline(&capture, &output, args.fps)?;

    Ok(())
}
```

### Library Code (thiserror)

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("device not found: {0}")]
    DeviceNotFound(String),

    #[error("failed to open device: {0}")]
    OpenFailed(#[from] std::io::Error),

    #[error("invalid resolution: {width}x{height}")]
    InvalidResolution { width: u32, height: u32 },

    #[error("capture timeout after {0}ms")]
    Timeout(u64),
}

pub fn open_device(path: &str) -> Result<Device, CaptureError> {
    // ... can return specific error variants
    Err(CaptureError::DeviceNotFound(path.to_string()))
}
```

## CLI Patterns with Clap

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input webcam device index
    #[arg(short, long, default_value_t = 0)]
    input_device: u32,

    /// Output v4l2loopback device path
    #[arg(short, long, default_value = "/dev/video10")]
    output_device: String,

    /// Capture resolution width
    #[arg(long, default_value_t = 1920)]
    capture_width: u32,

    /// Capture resolution height
    #[arg(long, default_value_t = 1080)]
    capture_height: u32,

    /// Target frames per second
    #[arg(long, default_value_t = 30)]
    fps: u32,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // args is now a type-safe struct
    println!("Input device: {}", args.input_device);
    println!("FPS: {}", args.fps);

    // ...
}
```

**Key Features:**
- Type-safe argument parsing
- Automatic help generation
- Default values
- Short and long flags
- Validation at parse time

## Logging with Tracing

### Basic Setup

```rust
fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)  // Hide module paths
        .init();

    tracing::info!("Application starting");
    tracing::debug!("Debug info: {:?}", some_value);

    // ...
}
```

### Performance Spans

```rust
fn process_frame(frame: &RgbImage) -> Result<Matte> {
    let _span = tracing::info_span!("preprocessing").entered();

    // Resize, normalize, etc.
    let preprocessed = resize_frame(frame)?;

    drop(_span);  // End preprocessing span

    let _span = tracing::info_span!("inference").entered();
    let matte = model.run(&preprocessed)?;

    Ok(matte)
}
```

### Structured Logging

```rust
tracing::info!(
    frame = frame_count,
    capture_ms = capture_time.as_secs_f64() * 1000.0,
    output_ms = output_time.as_secs_f64() * 1000.0,
    fps = actual_fps,
    "Frame processed"
);
```

## Frame Timing and Performance Monitoring

```rust
use std::time::{Duration, Instant};

fn run_pipeline<C, O>(capture: &mut C, output: &mut O, target_fps: u32) -> Result<()>
where
    C: CaptureSource,
    O: OutputSink,
{
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps as f32);
    let mut frame_count = 0u64;
    let mut total_capture_time = Duration::ZERO;
    let mut total_output_time = Duration::ZERO;

    tracing::info!("Starting main pipeline loop");

    loop {
        let loop_start = Instant::now();

        // Measure capture time
        let capture_start = Instant::now();
        let frame = capture.capture_frame()
            .context("Failed to capture frame")?;
        total_capture_time += capture_start.elapsed();

        // Measure output time
        let output_start = Instant::now();
        output.write_frame(&frame)
            .context("Failed to write frame")?;
        total_output_time += output_start.elapsed();

        frame_count += 1;

        // Log stats every 30 frames
        if frame_count % 30 == 0 {
            let avg_capture_ms = total_capture_time.as_secs_f64() * 1000.0 / frame_count as f64;
            let avg_output_ms = total_output_time.as_secs_f64() * 1000.0 / frame_count as f64;
            let total_ms = avg_capture_ms + avg_output_ms;
            let actual_fps = 1000.0 / total_ms;

            tracing::info!(
                "Frame {}: capture={:.1}ms, output={:.1}ms, total={:.1}ms, fps={:.1}",
                frame_count, avg_capture_ms, avg_output_ms, total_ms, actual_fps
            );
        }

        // Frame rate limiting
        let elapsed = loop_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }
}
```

**Key Techniques:**
- Accumulate timings across frames for accurate averages
- Log periodically (every 30 frames) to avoid log spam
- Calculate actual FPS from measured processing time
- Sleep only the remaining time in frame budget

## v4l2 Device Interaction

### Direct File Writing

```rust
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct V4L2Output {
    file: File,
    width: u32,
    height: u32,
}

impl V4L2Output {
    pub fn new<P: AsRef<Path>>(device_path: P, width: u32, height: u32) -> Result<Self> {
        let path = device_path.as_ref();

        tracing::info!(
            "Opening v4l2loopback device at {} ({}x{})",
            path.display(), width, height
        );

        // Open device file for writing
        let file = File::options()
            .write(true)
            .open(path)
            .with_context(|| format!("Failed to open {}", path.display()))?;

        Ok(Self { file, width, height })
    }
}

impl OutputSink for V4L2Output {
    fn write_frame(&mut self, frame: &RgbImage) -> Result<()> {
        // Resize if needed
        let frame = if frame.dimensions() != (self.width, self.height) {
            image::imageops::resize(
                frame,
                self.width,
                self.height,
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            frame.clone()
        };

        // Convert RGB to YUYV
        let yuyv_data = Self::rgb_to_yuyv(&frame);

        // Write directly to device
        self.file.write_all(&yuyv_data)
            .context("Failed to write frame to v4l2loopback device")?;

        Ok(())
    }

    fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
```

### RGB to YUYV Color Space Conversion

```rust
/// Convert RGB frame to YUYV (YUV 4:2:2) format
/// v4l2loopback typically expects YUYV
fn rgb_to_yuyv(rgb_image: &RgbImage) -> Vec<u8> {
    let (width, height) = rgb_image.dimensions();
    let mut yuyv = Vec::with_capacity((width * height * 2) as usize);

    for y in 0..height {
        for x in (0..width).step_by(2) {
            let pixel1 = rgb_image.get_pixel(x, y);
            let pixel2 = if x + 1 < width {
                rgb_image.get_pixel(x + 1, y)
            } else {
                pixel1
            };

            // Convert RGB to YUV
            let (y1, u1, v1) = rgb_to_yuv(pixel1[0], pixel1[1], pixel1[2]);
            let (y2, u2, v2) = rgb_to_yuv(pixel2[0], pixel2[1], pixel2[2]);

            // Average U and V for the pair
            let u = ((u1 as u16 + u2 as u16) / 2) as u8;
            let v = ((v1 as u16 + v2 as u16) / 2) as u8;

            // YUYV format: Y0 U Y1 V
            yuyv.push(y1);
            yuyv.push(u);
            yuyv.push(y2);
            yuyv.push(v);
        }
    }

    yuyv
}

fn rgb_to_yuv(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let r = r as f32;
    let g = g as f32;
    let b = b as f32;

    let y = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 255.0) as u8;
    let u = ((-0.147 * r - 0.289 * g + 0.436 * b) + 128.0).clamp(0.0, 255.0) as u8;
    let v = ((0.615 * r - 0.515 * g - 0.100 * b) + 128.0).clamp(0.0, 255.0) as u8;

    (y, u, v)
}
```

**Key Points:**
- v4l2loopback accepts raw frame data written to the device file
- YUYV is the most common format (4:2:2 chroma subsampling)
- Process pixels in pairs for U/V averaging
- Use proper color space conversion formulas

## Common Dependencies

| Crate | Purpose | Notes |
|-------|---------|-------|
| `anyhow` | Application error handling | Use in main.rs, CLI code |
| `thiserror` | Library error types | Use in reusable modules |
| `clap` | CLI argument parsing | Use derive feature |
| `tracing` | Structured logging | Better than `log` crate |
| `tracing-subscriber` | Log output formatting | Required with tracing |
| `image` | Image manipulation | RGB/RGBA operations, resize |
| `nokhwa` | Webcam capture | Cross-platform camera access |
| `v4l` | Linux video devices | v4l2 bindings |
| `ndarray` | N-dimensional arrays | For ML tensor operations |
| `ort` | ONNX Runtime | GPU-accelerated inference |

## Best Practices

### Module Organization

```rust
// src/capture/mod.rs - Public API
mod v4l_capture;
mod nokhwa_capture;

pub use v4l_capture::V4LCapture;
pub use nokhwa_capture::NokhwaCapture;

pub trait CaptureSource {
    fn capture_frame(&mut self) -> Result<RgbImage>;
}
```

### Avoid Allocations in Hot Loops

```rust
// Bad: allocates every frame
loop {
    let mut buffer = vec![0u8; width * height * 3];
    capture.read_into(&mut buffer)?;
}

// Good: reuse buffer
let mut buffer = vec![0u8; width * height * 3];
loop {
    capture.read_into(&mut buffer)?;
}
```

### Use `?` for Error Propagation

```rust
// Good
fn process() -> Result<Output> {
    let data = load_data()?;
    let processed = transform(data)?;
    Ok(processed)
}

// Avoid
fn process() -> Result<Output> {
    match load_data() {
        Ok(data) => match transform(data) {
            Ok(processed) => Ok(processed),
            Err(e) => Err(e),
        },
        Err(e) => Err(e),
    }
}
```

### Use Type Aliases for Clarity

```rust
type Frame = RgbImage;
type Matte = Vec<f32>;
type Resolution = (u32, u32);

fn process_frame(frame: Frame) -> Result<Matte> {
    // ...
}
```

## Common Patterns

### Builder Pattern

```rust
pub struct PipelineBuilder {
    capture_device: u32,
    output_device: String,
    target_fps: u32,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Self {
            capture_device: 0,
            output_device: "/dev/video10".to_string(),
            target_fps: 30,
        }
    }

    pub fn capture_device(mut self, device: u32) -> Self {
        self.capture_device = device;
        self
    }

    pub fn target_fps(mut self, fps: u32) -> Self {
        self.target_fps = fps;
        self
    }

    pub fn build(self) -> Result<Pipeline> {
        Pipeline::new(self.capture_device, &self.output_device, self.target_fps)
    }
}

// Usage
let pipeline = PipelineBuilder::new()
    .capture_device(1)
    .target_fps(60)
    .build()?;
```

### State Machine with Enums

```rust
enum PipelineState {
    Idle,
    Running { frame_count: u64 },
    Paused { at_frame: u64 },
    Error { message: String },
}

impl Pipeline {
    fn process_frame(&mut self) -> Result<()> {
        match &mut self.state {
            PipelineState::Running { frame_count } => {
                // Process frame
                *frame_count += 1;
                Ok(())
            }
            PipelineState::Paused { .. } => {
                // Skip processing
                Ok(())
            }
            _ => Err(anyhow!("Cannot process frame in current state")),
        }
    }
}
```

## Performance Tips

1. **Profile before optimizing**: Use `cargo flamegraph` or `perf`
2. **Avoid clones in hot loops**: Use references or move semantics
3. **Prefer stack allocation**: Use arrays over Vec when size is known
4. **Use release builds for benchmarking**: `cargo build --release`
5. **Consider rayon for parallelism**: Easy data parallelism with iterators

## Testing Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockCapture {
        width: u32,
        height: u32,
    }

    impl CaptureSource for MockCapture {
        fn capture_frame(&mut self) -> Result<RgbImage> {
            Ok(RgbImage::new(self.width, self.height))
        }

        fn resolution(&self) -> (u32, u32) {
            (self.width, self.height)
        }
    }

    #[test]
    fn test_pipeline_with_mock() {
        let mut capture = MockCapture { width: 640, height: 480 };
        let frame = capture.capture_frame().unwrap();
        assert_eq!(frame.dimensions(), (640, 480));
    }
}
```

## Common Pitfalls

### Forgetting to Handle Errors

```rust
// Wrong
let frame = capture.capture_frame().unwrap();

// Right
let frame = capture.capture_frame()
    .context("Failed to capture frame")?;
```

### Not Using Result for Fallible Operations

```rust
// Wrong
fn load_config() -> Config {
    // What if file doesn't exist?
}

// Right
fn load_config() -> Result<Config> {
    let contents = std::fs::read_to_string("config.toml")
        .context("Failed to read config file")?;
    // ...
}
```

### Blocking in Async Context (and vice versa)

```rust
// If your pipeline is sync, stay sync
// If it's async, use async all the way through
// Don't mix unless you know what you're doing
```

## Resources

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [docs.rs](https://docs.rs/) - Crate documentation
