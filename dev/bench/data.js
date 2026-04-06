window.BENCHMARK_DATA = {
  "lastUpdate": 1775500083102,
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
          "id": "3b161f50b72955fe9c1bba6d1a65641d38e5b86d",
          "message": "Merge pull request #12 from sn0w12/dependabot/cargo/bytes-1.11.1\n\nBump bytes from 1.10.1 to 1.11.1",
          "timestamp": "2026-04-06T17:52:41+02:00",
          "tree_id": "e492a1403860d63b3cd3b0dc3a3599df17d083f5",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/3b161f50b72955fe9c1bba6d1a65641d38e5b86d"
        },
        "date": 1775491081033,
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
            "value": 113,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 118,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 108,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/null_or_missing",
            "value": 73,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline_extraction_loop/extract_all_fields",
            "value": 747,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 116,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 119,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 118,
            "range": "± 0",
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
            "value": 335154,
            "range": "± 1431",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 402563,
            "range": "± 2151",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 205337,
            "range": "± 4011",
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
          "id": "2aded0392d9978a5b13b819669f44f70dfa909f1",
          "message": "Merge pull request #10 from sn0w12/dependabot/cargo/tempfile-3.27.0\n\nBump tempfile from 3.23.0 to 3.27.0",
          "timestamp": "2026-04-06T17:52:59+02:00",
          "tree_id": "e00d06ace0a9df852942e754dfb140408dbd3aa1",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/2aded0392d9978a5b13b819669f44f70dfa909f1"
        },
        "date": 1775491095247,
        "tool": "cargo",
        "benches": [
          {
            "name": "json_path_extraction/extract_json_value/flat",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/nested",
            "value": 120,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 123,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 113,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/null_or_missing",
            "value": 77,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline_extraction_loop/extract_all_fields",
            "value": 738,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 123,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 120,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 123,
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
            "value": 339664,
            "range": "± 3317",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 412522,
            "range": "± 5474",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 205302,
            "range": "± 3652",
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
          "id": "c454416eecb4c22255fa99651d8ab6928761aa81",
          "message": "Merge pull request #13 from sn0w12/dependabot/cargo/quinn-proto-0.11.14\n\nBump quinn-proto from 0.11.13 to 0.11.14",
          "timestamp": "2026-04-06T17:54:44+02:00",
          "tree_id": "1c0569a76052d287e51fc0d7fe6bddf370e60835",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/c454416eecb4c22255fa99651d8ab6928761aa81"
        },
        "date": 1775491196479,
        "tool": "cargo",
        "benches": [
          {
            "name": "json_path_extraction/extract_json_value/flat",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/nested",
            "value": 96,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 95,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 91,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/null_or_missing",
            "value": 61,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline_extraction_loop/extract_all_fields",
            "value": 575,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 97,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 102,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 101,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 305796,
            "range": "± 1389",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 373362,
            "range": "± 16761",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 202627,
            "range": "± 3238",
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
          "id": "eb71d82ca00802dcc81e5b79c7e9321b880898af",
          "message": "Merge pull request #14 from sn0w12/dependabot/cargo/rustls-webpki-0.103.10\n\nBump rustls-webpki from 0.103.8 to 0.103.10",
          "timestamp": "2026-04-06T17:54:36+02:00",
          "tree_id": "737070f0e53eb15222111886ba3f52ca9932940c",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/eb71d82ca00802dcc81e5b79c7e9321b880898af"
        },
        "date": 1775491197050,
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
            "value": 111,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 115,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 104,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/null_or_missing",
            "value": 69,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline_extraction_loop/extract_all_fields",
            "value": 683,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 113,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 121,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 114,
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
            "value": 338479,
            "range": "± 14192",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 412775,
            "range": "± 2570",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 224619,
            "range": "± 1160",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "48e5dfa751443763cd47728610ea67c1b25577d6",
          "message": "Split monitor into several smaller files",
          "timestamp": "2026-04-06T20:22:42+02:00",
          "tree_id": "1729b3b6b85f9fc2a0a5d788a7d337d14b38b0e2",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/48e5dfa751443763cd47728610ea67c1b25577d6"
        },
        "date": 1775500082769,
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
            "value": 124,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 121,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 115,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/null_or_missing",
            "value": 74,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline_extraction_loop/extract_all_fields",
            "value": 730,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 122,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 124,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 123,
            "range": "± 0",
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
            "value": 331459,
            "range": "± 2510",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 401362,
            "range": "± 1549",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 205447,
            "range": "± 7495",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}