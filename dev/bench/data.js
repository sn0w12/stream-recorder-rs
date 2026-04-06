window.BENCHMARK_DATA = {
  "lastUpdate": 1775491048201,
  "repoUrl": "https://github.com/sn0w12/stream-recorder-rs",
  "entries": {
    "Rust Benchmark": [
      {
        "commit": {
          "author": {
            "email": "lucasskold49@gmail.com",
            "name": "Sn0w123",
            "username": "sn0w12"
          },
          "committer": {
            "email": "lucasskold49@gmail.com",
            "name": "Sn0w123",
            "username": "sn0w12"
          },
          "distinct": true,
          "id": "e7883ad1dd27fb8f71b493d8e5cba69ad4adf10e",
          "message": "Fix bench not running properly",
          "timestamp": "2026-04-06T17:07:18+02:00",
          "tree_id": "dc145d194e41a6a1d9594016aad9e30bde63fb61",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/e7883ad1dd27fb8f71b493d8e5cba69ad4adf10e"
        },
        "date": 1775489066967,
        "tool": "cargo",
        "benches": [
          {
            "name": "json_path_extraction/extract_json_value/flat",
            "value": 47,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/nested",
            "value": 120,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 122,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 111,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/null_or_missing",
            "value": 74,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline_extraction_loop/extract_all_fields",
            "value": 744,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 126,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 124,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 124,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 330445,
            "range": "± 2069",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 402544,
            "range": "± 3102",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 196011,
            "range": "± 4574",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "105451515+sn0w12@users.noreply.github.com",
            "name": "sn0w",
            "username": "sn0w12"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a2b17128c6d3cb8356cd00984a6485be6f112932",
          "message": "Merge pull request #8 from sn0w12/dependabot/cargo/toml-1.1.2spec-1.1.0\n\nBump toml from 1.0.7+spec-1.1.0 to 1.1.2+spec-1.1.0",
          "timestamp": "2026-04-06T17:52:12+02:00",
          "tree_id": "02f40ed492490c145f9d99e42c55d53182687129",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/a2b17128c6d3cb8356cd00984a6485be6f112932"
        },
        "date": 1775491047763,
        "tool": "cargo",
        "benches": [
          {
            "name": "json_path_extraction/extract_json_value/flat",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/nested",
            "value": 119,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 115,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 106,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/null_or_missing",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline_extraction_loop/extract_all_fields",
            "value": 757,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 118,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 115,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 114,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 346014,
            "range": "± 4260",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 420505,
            "range": "± 4558",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 227690,
            "range": "± 1003",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}