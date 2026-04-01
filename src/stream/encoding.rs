use std::fmt;
use std::process::Stdio;

/// Selects how video is encoded during recording.
#[derive(Clone, Debug)]
pub enum VideoEncoding {
    /// Variable bitrate with a quality target (CRF/CQ, lower value → better quality, range 1–51).
    Quality(u32),
    /// Constant bitrate (e.g. `"6M"`, `"5000k"`).
    ConstantBitrate(String),
}

impl fmt::Display for VideoEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VideoEncoding::Quality(video_quality) => write!(f, "quality {}", video_quality),
            VideoEncoding::ConstantBitrate(bitrate) => write!(f, "constant bitrate {}", bitrate),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EncoderProfile {
    pub(crate) codec: String,
    pub(crate) options: Vec<String>,
}

impl EncoderProfile {
    fn new(codec: &str, options: Vec<String>) -> Self {
        Self {
            codec: codec.to_string(),
            options,
        }
    }
}

fn clamp_video_quality(video_quality: u32) -> u32 {
    video_quality.clamp(1, 51)
}

fn videotoolbox_quality_percent(video_quality: u32) -> u32 {
    let clamped = clamp_video_quality(video_quality);
    1 + ((51 - clamped) * 99) / 50
}

fn runtime_input_options(codec: &str) -> Vec<String> {
    match codec {
        "h264_qsv" | "hevc_qsv" => vec![
            "-hwaccel".into(),
            "qsv".into(),
            "-init_hw_device".into(),
            "qsv=hw".into(),
            "-filter_hw_device".into(),
            "hw".into(),
        ],
        "h264_nvenc" | "hevc_nvenc" => vec![
            "-hwaccel".into(),
            "cuda".into(),
            "-hwaccel_output_format".into(),
            "cuda".into(),
        ],
        "h264_vaapi" | "hevc_vaapi" => {
            vec!["-vaapi_device".into(), "/dev/dri/renderD128".into()]
        }
        _ => Vec::new(),
    }
}

fn probe_input_options(codec: &str) -> Vec<String> {
    match codec {
        "h264_vaapi" | "hevc_vaapi" => {
            vec!["-vaapi_device".into(), "/dev/dri/renderD128".into()]
        }
        _ => Vec::new(),
    }
}

fn build_hw_encoder_profiles(encoding: &VideoEncoding) -> Vec<EncoderProfile> {
    // Per-encoder rate-control flags differ between VBR (quality) and CBR (bitrate).
    // All other options (codec name, preset, pixel-format filters) are shared.
    let nvenc_rc_opts: Vec<String> = match encoding {
        VideoEncoding::Quality(q) => {
            let quality = clamp_video_quality(*q).to_string();
            vec![
                "-rc".into(),
                "vbr".into(),
                "-cq".into(),
                quality,
                "-b:v".into(),
                "0".into(),
            ]
        }
        VideoEncoding::ConstantBitrate(b) => {
            vec!["-rc".into(), "cbr".into(), "-b:v".into(), b.clone()]
        }
    };

    let qsv_rate_opts: Vec<String> = match encoding {
        VideoEncoding::Quality(q) => {
            vec![
                "-global_quality".into(),
                clamp_video_quality(*q).to_string(),
            ]
        }
        VideoEncoding::ConstantBitrate(b) => vec!["-b:v".into(), b.clone()],
    };

    let vaapi_rc_opts: Vec<String> = match encoding {
        VideoEncoding::Quality(q) => {
            vec![
                "-rc_mode".into(),
                "QVBR".into(),
                "-global_quality".into(),
                clamp_video_quality(*q).to_string(),
            ]
        }
        VideoEncoding::ConstantBitrate(b) => {
            vec!["-rc_mode".into(), "CBR".into(), "-b:v".into(), b.clone()]
        }
    };

    let amf_rc_opts: Vec<String> = match encoding {
        VideoEncoding::Quality(q) => {
            vec![
                "-rc".into(),
                "qvbr".into(),
                "-qvbr_quality_level".into(),
                clamp_video_quality(*q).to_string(),
            ]
        }
        VideoEncoding::ConstantBitrate(b) => {
            vec!["-rc".into(), "cbr".into(), "-b:v".into(), b.clone()]
        }
    };

    let vt_rate_opts: Vec<String> = match encoding {
        VideoEncoding::Quality(q) => {
            let vt_quality = videotoolbox_quality_percent(clamp_video_quality(*q)).to_string();
            vec!["-global_quality".into(), vt_quality]
        }
        VideoEncoding::ConstantBitrate(b) => vec!["-b:v".into(), b.clone()],
    };

    let omx_rate_opts: Vec<String> = match encoding {
        VideoEncoding::Quality(_) => vec![],
        VideoEncoding::ConstantBitrate(b) => vec!["-b:v".into(), b.clone()],
    };

    vec![
        EncoderProfile::new("h264_nvenc", {
            let mut opts = vec![
                "-c:v".into(),
                "h264_nvenc".into(),
                "-preset".into(),
                "p4".into(),
            ];
            opts.extend(nvenc_rc_opts.clone());
            opts
        }),
        EncoderProfile::new("hevc_nvenc", {
            let mut opts = vec![
                "-c:v".into(),
                "hevc_nvenc".into(),
                "-preset".into(),
                "p4".into(),
            ];
            opts.extend(nvenc_rc_opts);
            opts
        }),
        EncoderProfile::new("h264_qsv", {
            let mut opts = vec![
                "-c:v".into(),
                "h264_qsv".into(),
                "-preset".into(),
                "medium".into(),
            ];
            opts.extend(qsv_rate_opts.clone());
            opts
        }),
        EncoderProfile::new("hevc_qsv", {
            let mut opts = vec![
                "-c:v".into(),
                "hevc_qsv".into(),
                "-preset".into(),
                "medium".into(),
            ];
            opts.extend(qsv_rate_opts);
            opts
        }),
        EncoderProfile::new("h264_vaapi", {
            let mut opts = vec![
                "-vf".into(),
                "format=nv12,hwupload".into(),
                "-c:v".into(),
                "h264_vaapi".into(),
            ];
            opts.extend(vaapi_rc_opts.clone());
            opts
        }),
        EncoderProfile::new("hevc_vaapi", {
            let mut opts = vec![
                "-vf".into(),
                "format=nv12,hwupload".into(),
                "-c:v".into(),
                "hevc_vaapi".into(),
            ];
            opts.extend(vaapi_rc_opts);
            opts
        }),
        EncoderProfile::new("h264_amf", {
            let mut opts = vec![
                "-c:v".into(),
                "h264_amf".into(),
                "-usage".into(),
                "transcoding".into(),
                "-quality".into(),
                "quality".into(),
            ];
            opts.extend(amf_rc_opts.clone());
            opts
        }),
        EncoderProfile::new("hevc_amf", {
            let mut opts = vec![
                "-c:v".into(),
                "hevc_amf".into(),
                "-usage".into(),
                "transcoding".into(),
                "-quality".into(),
                "quality".into(),
            ];
            opts.extend(amf_rc_opts);
            opts
        }),
        EncoderProfile::new("h264_videotoolbox", {
            let mut opts = vec!["-c:v".into(), "h264_videotoolbox".into()];
            opts.extend(vt_rate_opts);
            opts
        }),
        EncoderProfile::new("h264_omx", {
            let mut opts = vec!["-c:v".into(), "h264_omx".into()];
            opts.extend(omx_rate_opts);
            opts
        }),
    ]
}

// Run a short ffmpeg probe to verify that the encoder profile actually works at runtime.
// Many builds list encoders at compile-time (`ffmpeg -encoders`) even when the
// hardware/driver isn't present; this runtime probe prevents selecting a broken
// encoder that immediately exits and produces 0-length files.
async fn verify_hw_encoder(profile: &EncoderProfile) -> Result<(), String> {
    // Run a short ffmpeg probe and capture stderr for diagnostics.
    let mut args = vec!["-hide_banner".into(), "-loglevel".into(), "error".into()];
    args.extend(probe_input_options(&profile.codec));
    args.extend(vec![
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        "testsrc=duration=1:size=640x360:rate=30".into(),
    ]);
    args.extend(profile.options.clone());
    args.extend(vec![
        "-t".into(),
        "1".into(),
        "-f".into(),
        "null".into(),
        "-".into(),
    ]);

    let probe = tokio::process::Command::new("ffmpeg")
        .args(&args)
        .stderr(Stdio::piped())
        .output()
        .await;

    // Helper to extract a short reason from ffmpeg stderr
    fn short_reason(stderr: &str) -> String {
        let s = stderr.to_lowercase();
        if s.contains("cuda_error_no_device")
            || s.contains("cuinit(0) failed")
            || s.contains("no cuda")
        {
            return "no CUDA-capable device".into();
        }
        if s.contains("error creating a mfx session") || s.contains("mfx") {
            return "intel qsv: mfx session not available".into();
        }
        if s.contains("dll amfrt64.dll failed to open") || s.contains("amfrt64.dll") {
            return "amd amf runtime not found".into();
        }
        if s.contains("error while opening encoder") {
            return "encoder failed to open (bad params or missing runtime)".into();
        }
        if s.contains("nothing was written into output file") || s.contains("received no packets") {
            return "encoder produced no output packets".into();
        }
        // Fallback: return the first non-empty ffmpeg stderr line (trimmed)
        stderr
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim().to_string())
            .unwrap_or_else(|| "unknown error".into())
    }

    match probe {
        Ok(output) => {
            if output.status.success() {
                return Ok(());
            }
            let err = String::from_utf8_lossy(&output.stderr);
            Err(short_reason(&err))
        }
        Err(err) => Err(format!("failed to run ffmpeg: {}", err)),
    }
}

/// Detects the best available hardware encoder by querying ffmpeg at runtime and
/// verifying it works. Priority: NVENC → QSV → VAAPI → AMF → VideoToolbox → OMX.
/// Returns the encoder `-c:v` name plus encoder-specific ffmpeg options, or
/// `None` if no working hardware encoder is found.
pub async fn detect_best_hw_encoder(encoding: &VideoEncoding) -> Option<(String, Vec<String>)> {
    // First check the build-time availability to avoid unnecessarily probing
    // encoders that aren't compiled into ffmpeg.
    let encoders_out = match tokio::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
        .await
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_lowercase(),
        Err(_) => String::new(),
    };

    let profiles = build_hw_encoder_profiles(encoding);

    for profile in profiles {
        if !encoders_out.contains(&profile.codec) {
            continue; // not present in this ffmpeg build
        }

        // runtime verification — some encoders are listed but not usable at runtime
        match verify_hw_encoder(&profile).await {
            Ok(()) => return Some((profile.codec, profile.options)),
            Err(_reason) => continue,
        }
    }

    None
}

/// Public helper to probe available hw encoders and print diagnostics.
/// This is intended for the CLI `encoders test` command.
pub async fn probe_hw_encoders() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // print ffmpeg encoder build-time list filtered for hardware encoders
    let encoders_out = match tokio::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
        .await
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(e) => {
            eprintln!("failed to run 'ffmpeg -encoders': {}", e);
            String::new()
        }
    };

    println!("ffmpeg -encoders (hardware-related lines):");
    for line in encoders_out.lines() {
        let l = line.to_lowercase();
        if l.contains("nvenc")
            || l.contains("qsv")
            || l.contains("amf")
            || l.contains("vaapi")
            || l.contains("videotoolbox")
            || l.contains("omx")
            || l.contains("v4l2m2m")
        {
            println!("  {}", line.trim());
        }
    }

    // candidate encoders we probe (same order as detection)
    println!("\nRuntime probe for each candidate (1s test):");
    for profile in build_hw_encoder_profiles(&VideoEncoding::Quality(23)) {
        if !encoders_out.to_lowercase().contains(&profile.codec) {
            println!("  {:20} — not compiled into ffmpeg", profile.codec);
            continue;
        }

        print!("  {:20} — probing... ", profile.codec);
        match verify_hw_encoder(&profile).await {
            Ok(()) => println!("OK"),
            Err(reason) => println!("FAIL ({})", reason),
        }
    }

    // final selection using detect_best_hw_encoder
    match detect_best_hw_encoder(&VideoEncoding::Quality(23)).await {
        Some((enc, _opts)) => println!("\nSelected encoder: {}", enc),
        None => {
            println!("\nNo working hardware encoder detected; software (libx264) will be used.")
        }
    }

    Ok(())
}

/// Builds ffmpeg arguments for recording with hardware acceleration when available.
pub fn build_ffmpeg_args(
    playback_url: &str,
    output_path: &str,
    encoding: &VideoEncoding,
    hw_encoder: Option<(String, Vec<String>)>,
    max_bitrate: Option<&str>,
) -> Vec<String> {
    let mut ffmpeg_args: Vec<String> = vec!["-loglevel".into(), "quiet".into()];

    if let Some((codec, opts)) = hw_encoder {
        ffmpeg_args.extend(runtime_input_options(&codec));
        ffmpeg_args.push("-i".into());
        ffmpeg_args.push(playback_url.to_string());
        ffmpeg_args.extend(opts);
    } else {
        match encoding {
            VideoEncoding::Quality(video_quality) => {
                let quality = clamp_video_quality(*video_quality).to_string();
                println!("No hardware encoder available, using variable bitrate software encoding");
                ffmpeg_args.push("-i".into());
                ffmpeg_args.push(playback_url.to_string());
                ffmpeg_args.extend(vec![
                    "-c:v".into(),
                    "libx264".into(),
                    "-preset".into(),
                    "veryfast".into(),
                    "-crf".into(),
                    quality,
                ]);
            }
            VideoEncoding::ConstantBitrate(bitrate) => {
                println!("No hardware encoder available, using constant bitrate software encoding");
                ffmpeg_args.push("-i".into());
                ffmpeg_args.push(playback_url.to_string());
                ffmpeg_args.extend(vec![
                    "-c:v".into(),
                    "libx264".into(),
                    "-preset".into(),
                    "veryfast".into(),
                    "-b:v".into(),
                    bitrate.clone(),
                    "-maxrate".into(),
                    bitrate.clone(),
                    "-bufsize".into(),
                    bitrate.clone(),
                ]);
            }
        }
    }

    // Apply optional max bitrate cap
    if let Some(rate) = max_bitrate {
        ffmpeg_args.extend(vec![
            "-maxrate".into(),
            rate.to_string(),
            "-bufsize".into(),
            rate.to_string(),
        ]);
    }

    // Add audio encoding and output path
    ffmpeg_args.extend(vec![
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        "128k".into(),
        output_path.to_string(),
    ]);

    ffmpeg_args
}

#[cfg(test)]
mod tests {
    use super::{
        VideoEncoding, build_ffmpeg_args, build_hw_encoder_profiles, videotoolbox_quality_percent,
    };

    #[test]
    fn videotoolbox_quality_mapping_prefers_lower_quality_values() {
        assert_eq!(videotoolbox_quality_percent(1), 100);
        assert_eq!(videotoolbox_quality_percent(23), 56);
        assert_eq!(videotoolbox_quality_percent(51), 1);
        assert_eq!(videotoolbox_quality_percent(99), 1);
    }

    #[test]
    fn build_ffmpeg_args_software_fallback_uses_crf_quality() {
        let args = build_ffmpeg_args(
            "https://example.com/live.m3u8",
            "output.mp4",
            &VideoEncoding::Quality(19),
            None,
            None,
        );

        assert!(args.windows(2).any(|window| window == ["-c:v", "libx264"]));
        assert!(args.windows(2).any(|window| window == ["-crf", "19"]));
        assert!(!args.iter().any(|arg| arg == "-b:v"));
        assert!(!args.iter().any(|arg| arg == "-maxrate"));
    }

    #[test]
    fn build_ffmpeg_args_with_max_bitrate() {
        let args = build_ffmpeg_args(
            "https://example.com/live.m3u8",
            "output.mp4",
            &VideoEncoding::Quality(19),
            None,
            Some("6M"),
        );

        assert!(args.windows(2).any(|window| window == ["-maxrate", "6M"]));
        assert!(args.windows(2).any(|window| window == ["-bufsize", "6M"]));
    }

    #[test]
    fn build_ffmpeg_args_cbr_software_uses_bitrate_flags() {
        let args = build_ffmpeg_args(
            "https://example.com/live.m3u8",
            "output.mp4",
            &VideoEncoding::ConstantBitrate("6M".to_string()),
            None,
            None,
        );

        assert!(args.windows(2).any(|window| window == ["-c:v", "libx264"]));
        assert!(args.windows(2).any(|window| window == ["-b:v", "6M"]));
        assert!(args.windows(2).any(|window| window == ["-maxrate", "6M"]));
        assert!(args.windows(2).any(|window| window == ["-bufsize", "6M"]));
        assert!(!args.iter().any(|arg| arg == "-crf"));
    }

    #[test]
    fn build_hw_encoder_profiles_include_backend_specific_quality_flags() {
        let profiles = build_hw_encoder_profiles(&VideoEncoding::Quality(23));

        let nvenc = profiles
            .iter()
            .find(|profile| profile.codec == "h264_nvenc")
            .expect("nvenc profile should exist");
        assert!(
            nvenc
                .options
                .windows(2)
                .any(|window| window == ["-rc", "vbr"])
        );
        assert!(
            nvenc
                .options
                .windows(2)
                .any(|window| window == ["-cq", "23"])
        );
        assert!(
            nvenc
                .options
                .windows(2)
                .any(|window| window == ["-b:v", "0"])
        );

        let qsv = profiles
            .iter()
            .find(|profile| profile.codec == "h264_qsv")
            .expect("qsv profile should exist");
        assert!(
            qsv.options
                .windows(2)
                .any(|window| window == ["-global_quality", "23"])
        );

        let amf = profiles
            .iter()
            .find(|profile| profile.codec == "h264_amf")
            .expect("amf profile should exist");
        assert!(
            amf.options
                .windows(2)
                .any(|window| window == ["-rc", "qvbr"])
        );
        assert!(
            amf.options
                .windows(2)
                .any(|window| window == ["-qvbr_quality_level", "23"])
        );
    }

    #[test]
    fn build_cbr_hw_encoder_profiles_include_bitrate_flags() {
        let profiles = build_hw_encoder_profiles(&VideoEncoding::ConstantBitrate("6M".to_string()));

        let nvenc = profiles
            .iter()
            .find(|profile| profile.codec == "h264_nvenc")
            .expect("nvenc profile should exist");
        assert!(
            nvenc
                .options
                .windows(2)
                .any(|window| window == ["-rc", "cbr"])
        );
        assert!(
            nvenc
                .options
                .windows(2)
                .any(|window| window == ["-b:v", "6M"])
        );

        let vaapi = profiles
            .iter()
            .find(|profile| profile.codec == "h264_vaapi")
            .expect("vaapi profile should exist");
        assert!(
            vaapi
                .options
                .windows(2)
                .any(|window| window == ["-rc_mode", "CBR"])
        );
        assert!(
            vaapi
                .options
                .windows(2)
                .any(|window| window == ["-b:v", "6M"])
        );

        let amf = profiles
            .iter()
            .find(|profile| profile.codec == "h264_amf")
            .expect("amf profile should exist");
        assert!(
            amf.options
                .windows(2)
                .any(|window| window == ["-rc", "cbr"])
        );
        assert!(
            amf.options
                .windows(2)
                .any(|window| window == ["-b:v", "6M"])
        );
    }
}
