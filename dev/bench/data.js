window.BENCHMARK_DATA = {
  "lastUpdate": 1776543879977,
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
        "date": 1775500546122,
        "tool": "cargo",
        "benches": [
          {
            "name": "json_path_extraction/extract_json_value/flat",
            "value": 43,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/nested",
            "value": 123,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 116,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 117,
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
            "value": 755,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 128,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 133,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 127,
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
            "value": 336761,
            "range": "± 8211",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 403140,
            "range": "± 3745",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 201931,
            "range": "± 1546",
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
          "id": "4e883e9c25a451c5e247d3f7cb4a7fa2b1503718",
          "message": "Make handle_list_monitors its own function",
          "timestamp": "2026-04-06T20:42:53+02:00",
          "tree_id": "c85f2a18585020c7e6af69057b3db6e25dfa06ae",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/4e883e9c25a451c5e247d3f7cb4a7fa2b1503718"
        },
        "date": 1775501294427,
        "tool": "cargo",
        "benches": [
          {
            "name": "json_path_extraction/extract_json_value/flat",
            "value": 43,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/nested",
            "value": 112,
            "range": "± 0",
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
            "value": 727,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 112,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 117,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 113,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 334695,
            "range": "± 1540",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 403814,
            "range": "± 2011",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 229448,
            "range": "± 23070",
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
          "id": "c54773c351f01fd50ca085c1216a7855202087de",
          "message": "simplify post-processing",
          "timestamp": "2026-04-06T21:33:08+02:00",
          "tree_id": "8e30715ee6589329566e07695d18fa1aa31e9674",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/c54773c351f01fd50ca085c1216a7855202087de"
        },
        "date": 1775504304863,
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
            "value": 118,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 117,
            "range": "± 1",
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
            "value": 691,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 115,
            "range": "± 1",
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
            "value": 118,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 334357,
            "range": "± 2798",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 406200,
            "range": "± 12527",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 229492,
            "range": "± 1758",
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
          "id": "435374de993efea14ee693fa53a2e11c0a90e3d8",
          "message": "Add global title_clean_regex config",
          "timestamp": "2026-04-08T13:40:04+02:00",
          "tree_id": "6481c54d3c732fcee6dd3236fa304fd13d6cb0e9",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/435374de993efea14ee693fa53a2e11c0a90e3d8"
        },
        "date": 1775648730208,
        "tool": "cargo",
        "benches": [
          {
            "name": "json_path_extraction/extract_json_value/flat",
            "value": 36,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/nested",
            "value": 96,
            "range": "± 2",
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
            "value": 571,
            "range": "± 3",
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
            "value": 99,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 97,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 305310,
            "range": "± 2446",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 366206,
            "range": "± 1819",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 203156,
            "range": "± 496",
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
          "id": "eeb7473129a9574087d293aec28e96eaf7aad667",
          "message": "Update alert-threshold",
          "timestamp": "2026-04-08T13:48:04+02:00",
          "tree_id": "b529790bc9e035b623757854b3768b4f4c403999",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/eeb7473129a9574087d293aec28e96eaf7aad667"
        },
        "date": 1775649203434,
        "tool": "cargo",
        "benches": [
          {
            "name": "json_path_extraction/extract_json_value/flat",
            "value": 36,
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
            "value": 90,
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
            "value": 567,
            "range": "± 27",
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
            "value": 98,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 97,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 303848,
            "range": "± 2340",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 364286,
            "range": "± 11079",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 202792,
            "range": "± 5531",
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
          "id": "c755f18d312f950c4b05a9d64cfb5039d114f406",
          "message": "Move thumb file to postprocess",
          "timestamp": "2026-04-08T15:29:53+02:00",
          "tree_id": "a05f98078edb19994b41ce961b3de0902e553568",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/c755f18d312f950c4b05a9d64cfb5039d114f406"
        },
        "date": 1775655349119,
        "tool": "cargo",
        "benches": [
          {
            "name": "json_path_extraction/extract_json_value/flat",
            "value": 43,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/nested",
            "value": 118,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 123,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 107,
            "range": "± 1",
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
            "value": 746,
            "range": "± 4",
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
            "value": 117,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 115,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 23,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 339651,
            "range": "± 1946",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 410086,
            "range": "± 3321",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 193035,
            "range": "± 1431",
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
          "id": "3fedbc7d95c98db4dbea1a45f651fb7b485ff3bb",
          "message": "Improve docs slightly",
          "timestamp": "2026-04-08T19:16:24+02:00",
          "tree_id": "a7af414732539d7e1f127d238a8fb60d3d6d5b78",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/3fedbc7d95c98db4dbea1a45f651fb7b485ff3bb"
        },
        "date": 1775668910772,
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
            "value": 120,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 117,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 111,
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
            "value": 740,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 120,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 125,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 122,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 343284,
            "range": "± 3324",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 414306,
            "range": "± 2876",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 194056,
            "range": "± 915",
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
          "id": "7a1b2062092e4558aacd052e11af673e4758b191",
          "message": "Add thumbnail cli commands",
          "timestamp": "2026-04-15T01:39:43+02:00",
          "tree_id": "38f9933c2a57939debcbdef2789bdce364b045a7",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/7a1b2062092e4558aacd052e11af673e4758b191"
        },
        "date": 1776210289155,
        "tool": "cargo",
        "benches": [
          {
            "name": "json_path_extraction/extract_json_value/flat",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/nested",
            "value": 96,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 97,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 91,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/null_or_missing",
            "value": 65,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline_extraction_loop/extract_all_fields",
            "value": 573,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 96,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 100,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 99,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 307724,
            "range": "± 4487",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 369258,
            "range": "± 2153",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 187637,
            "range": "± 1277",
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
          "id": "359bebd7048713fe91780c23fd16a68727d7962c",
          "message": "Add stream_metadata_refresh_interval",
          "timestamp": "2026-04-18T16:34:58+02:00",
          "tree_id": "d974cf279948ed7c3bc23fae8709f9d8dc2a8293",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/359bebd7048713fe91780c23fd16a68727d7962c"
        },
        "date": 1776523198674,
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
            "value": 122,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 119,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 112,
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
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 121,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 125,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 120,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 333607,
            "range": "± 3460",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 406648,
            "range": "± 2483",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 225797,
            "range": "± 14127",
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
          "id": "e804ee6da7a4ec3af5a4f52ea2d1d317b4c1b38d",
          "message": "Always keep config key fully visible in table",
          "timestamp": "2026-04-18T18:51:42+02:00",
          "tree_id": "8173d000b94360f3004b3b74d9c6057222daeffb",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/e804ee6da7a4ec3af5a4f52ea2d1d317b4c1b38d"
        },
        "date": 1776531403444,
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
            "value": 125,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 123,
            "range": "± 1",
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
            "value": 75,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline_extraction_loop/extract_all_fields",
            "value": 718,
            "range": "± 4",
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
            "value": 127,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 121,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 333324,
            "range": "± 1231",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 403318,
            "range": "± 2198",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 202381,
            "range": "± 571",
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
          "id": "511f5d2a76c6fec5821470973edc8531e07238ff",
          "message": "Update tests",
          "timestamp": "2026-04-18T19:20:17+02:00",
          "tree_id": "fa432eefecb0b3c1f3a1beb35b7d8fcc58e517fc",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/511f5d2a76c6fec5821470973edc8531e07238ff"
        },
        "date": 1776533122566,
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
            "value": 123,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 116,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 115,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/null_or_missing",
            "value": 75,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pipeline_extraction_loop/extract_all_fields",
            "value": 741,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 128,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 131,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 334769,
            "range": "± 2200",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 406029,
            "range": "± 3028",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 203639,
            "range": "± 731",
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
          "id": "fc163ec9d074b396e223d8ed93b947d86d5437e3",
          "message": "Update config docs",
          "timestamp": "2026-04-18T22:00:37+02:00",
          "tree_id": "4f450dfb91d0fe7f79afdf99ce6ce8f78e3b6c49",
          "url": "https://github.com/sn0w12/stream-recorder-rs/commit/fc163ec9d074b396e223d8ed93b947d86d5437e3"
        },
        "date": 1776543879564,
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
            "value": 112,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/array",
            "value": 108,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "json_path_extraction/extract_json_value/missing",
            "value": 114,
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
            "value": 693,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/path_exists",
            "value": 113,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_match",
            "value": 118,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "live_check_matching/matches/equals_miss",
            "value": 115,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/no_rules",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/one_rule",
            "value": 334990,
            "range": "± 1266",
            "unit": "ns/iter"
          },
          {
            "name": "title_cleaning/clean_title/three_rules",
            "value": 404881,
            "range": "± 2902",
            "unit": "ns/iter"
          },
          {
            "name": "template_rendering/example_template",
            "value": 224809,
            "range": "± 3884",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}