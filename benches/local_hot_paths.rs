use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use serde_json::json;
use std::collections::HashMap;

use stream_recorder::platform::{
    LiveCheck, LiveCheckCondition, PipelineStep, PlatformConfig, TitleCleanRule, extract_json_value,
};
use stream_recorder::template::{TemplateValue, render_template};

fn sample_platform(title_clean_regex: Option<Vec<TitleCleanRule>>) -> PlatformConfig {
    PlatformConfig {
        id: "sample".to_string(),
        name: "Sample".to_string(),
        base_url: "https://example.com/api/".to_string(),
        icon: None,
        token_name: None,
        headers: HashMap::new(),
        steps: vec![PipelineStep {
            endpoint: "stream/{username}".to_string(),
            live_check: None,
            extract: HashMap::new(),
        }],
        source_url: None,
        version: "1.0.0".to_string(),
        stream_recorder_version: None,
        title_clean_regex,
    }
}

fn sample_response() -> serde_json::Value {
    json!({
        "id": "stream-123",
        "response": {
            "stream": {
                "playbackUrl": "https://example.com/hls.m3u8",
                "status": "live",
                "title": " :smile: Creator Stream [VOD] ",
                "owner": {
                    "name": "creator",
                    "id": 42
                }
            },
            "items": [
                {"id": "alpha", "value": 1},
                {"id": "beta", "value": 2},
                {"id": "gamma", "value": 3}
            ]
        },
        "flags": {
            "live": true,
            "featured": false
        }
    })
}

fn sample_pipeline_extractors() -> HashMap<String, String> {
    HashMap::from([
        (
            "playback_url".to_string(),
            "response.stream.playbackUrl".to_string(),
        ),
        (
            "stream_title".to_string(),
            "response.stream.title".to_string(),
        ),
        (
            "user_id".to_string(),
            "response.stream.owner.id".to_string(),
        ),
        (
            "first_item_id".to_string(),
            "response.items[0].id".to_string(),
        ),
        (
            "second_item_value".to_string(),
            "response.items[1].value".to_string(),
        ),
        ("live_flag".to_string(), "flags.live".to_string()),
    ])
}

fn sample_context() -> HashMap<String, TemplateValue> {
    HashMap::from([
        (
            "date".to_string(),
            TemplateValue::String("2025-11-09".to_string()),
        ),
        (
            "username".to_string(),
            TemplateValue::String("example_user".to_string()),
        ),
        (
            "user_id".to_string(),
            TemplateValue::String("12345".to_string()),
        ),
        (
            "output_path".to_string(),
            TemplateValue::String("/path/to/recording.mp4".to_string()),
        ),
        (
            "thumbnail_path".to_string(),
            TemplateValue::String("/path/to/thumbnail.jpg".to_string()),
        ),
        (
            "stream_title".to_string(),
            TemplateValue::String("Example Stream Title".to_string()),
        ),
        (
            "bunkr_urls".to_string(),
            TemplateValue::Array(vec![
                "https://bunkr.example.com/file1".to_string(),
                "https://bunkr.example.com/file2".to_string(),
            ]),
        ),
        (
            "gofile_urls".to_string(),
            TemplateValue::Array(vec!["https://gofile.example.com/download".to_string()]),
        ),
        (
            "fileditch_urls".to_string(),
            TemplateValue::Array(vec!["https://fileditch.example.com/file".to_string()]),
        ),
        (
            "filester_urls".to_string(),
            TemplateValue::Array(vec!["https://filester.example.com/file".to_string()]),
        ),
    ])
}

fn sample_template() -> &'static str {
    include_str!("../templates/example.hbr")
}

fn bench_json_path_extraction(c: &mut Criterion) {
    let value = sample_response();
    let mut group = c.benchmark_group("json_path_extraction");

    for (name, path) in [
        ("flat", "id"),
        ("nested", "response.stream.playbackUrl"),
        ("array", "response.items[1].value"),
        ("missing", "response.stream.missing"),
        ("null_or_missing", "flags.missing"),
    ] {
        group.bench_with_input(
            BenchmarkId::new("extract_json_value", name),
            &path,
            |b, &json_path| {
                b.iter(|| black_box(extract_json_value(black_box(&value), black_box(json_path))))
            },
        );
    }

    group.finish();
}

fn bench_pipeline_extraction_loop(c: &mut Criterion) {
    let value = sample_response();
    let extractors = sample_pipeline_extractors();
    let mut group = c.benchmark_group("pipeline_extraction_loop");

    group.bench_function("extract_all_fields", |b| {
        b.iter(|| {
            let mut extracted = 0usize;
            for json_path in extractors.values() {
                if let Some(text) = extract_json_value(black_box(&value), black_box(json_path))
                    .and_then(|v| v.as_str())
                {
                    extracted += text.len();
                }
            }
            black_box(extracted)
        })
    });

    group.finish();
}

fn bench_live_check_matching(c: &mut Criterion) {
    let value = sample_response();
    let mut group = c.benchmark_group("live_check_matching");

    let checks = [
        (
            "path_exists",
            LiveCheck::Path("response.stream.playbackUrl".to_string()),
        ),
        (
            "equals_match",
            LiveCheck::Condition(LiveCheckCondition {
                path: "response.stream.status".to_string(),
                exists: None,
                equals: Some(json!("live")),
                not_equals: None,
            }),
        ),
        (
            "equals_miss",
            LiveCheck::Condition(LiveCheckCondition {
                path: "response.stream.status".to_string(),
                exists: None,
                equals: Some(json!("offline")),
                not_equals: None,
            }),
        ),
    ];

    for (name, check) in checks {
        group.bench_with_input(BenchmarkId::new("matches", name), &check, |b, check| {
            b.iter(|| black_box(check).matches(black_box(&value)))
        });
    }

    group.finish();
}

fn bench_title_cleaning(c: &mut Criterion) {
    let mut group = c.benchmark_group("title_cleaning");
    let title = "  :smile: Hello   [VOD]   World :tada:  ";

    let no_rules = sample_platform(None);
    let one_rule = sample_platform(Some(vec![TitleCleanRule::Pattern(r":\w+:".to_string())]));
    let multiple_rules = sample_platform(Some(vec![
        TitleCleanRule::Pattern(r":\w+:".to_string()),
        TitleCleanRule::Pattern(r"\[VOD\]".to_string()),
        TitleCleanRule::Replace {
            pattern: r"\s+".to_string(),
            replacement: " ".to_string(),
        },
    ]));

    for (name, platform) in [
        ("no_rules", no_rules),
        ("one_rule", one_rule),
        ("three_rules", multiple_rules),
    ] {
        group.bench_with_input(
            BenchmarkId::new("clean_title", name),
            &platform,
            |b, platform| b.iter(|| black_box(platform).clean_title(black_box(title))),
        );
    }

    group.finish();
}

fn bench_template_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("template_rendering");
    let template = sample_template();
    let context = sample_context();

    group.bench_function("example_template", |b| {
        b.iter(|| render_template(black_box(template), black_box(&context)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_json_path_extraction,
    bench_pipeline_extraction_loop,
    bench_live_check_matching,
    bench_title_cleaning,
    bench_template_rendering
);
criterion_main!(benches);
