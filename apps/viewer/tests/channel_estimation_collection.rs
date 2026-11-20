/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
use std::sync::atomic::AtomicBool;
use viewer::{
    capture::collection::write_json_new,
    measurements::channel_estimation::collection::{self, Collection, ImportLimits},
};
fn capture(epoch: u32, channel: u8) -> Vec<u8> {
    let mut bytes = pnm_fixtures::CHANNEL_ESTIMATE.to_vec();
    bytes[6..10].copy_from_slice(&epoch.to_be_bytes());
    bytes[10] = channel;
    bytes
}
#[test]
fn folder_import_groups_by_binary_headers_sorts_and_exports_the_full_mean() {
    let dir = tempfile::tempdir().unwrap();
    for (name, data) in [
        ("z.bin", capture(10, 34)),
        ("a.bin", capture(20, 34)),
        ("duplicate.bin", capture(10, 34)),
        ("wrong-name.bin", capture(30, 35)),
        ("wrong-type.bin", pnm_fixtures::RXMER.to_vec()),
    ] {
        std::fs::write(dir.path().join(name), data).unwrap();
    }
    let collection =
        Collection::scan(dir.path(), ImportLimits::default(), &AtomicBool::new(false)).unwrap();
    assert_eq!(collection.catalog.accepted_count, 3);
    assert_eq!(collection.catalog.duplicate_count, 1);
    assert_eq!(collection.catalog.skipped_count, 1);
    assert_eq!(collection.catalog.groups.len(), 2);
    let result = collection.aggregate(0).unwrap();
    assert_eq!(
        result
            .sources
            .iter()
            .map(|s| s.capture_time)
            .collect::<Vec<_>>(),
        vec![10, 20]
    );
    assert_eq!(result.average_iq.len(), 7480);
    let bounds = collection.playback_magnitude_bounds(0).unwrap();
    let reference = collection::viewport(&result, false, 0, 7480).unwrap();
    assert_eq!(bounds.0, reference["minimum"].as_f64().unwrap());
    assert_eq!(bounds.1, reference["maximum"].as_f64().unwrap());
    assert!(bounds.0 < bounds.1);
    assert!(collection.playback_magnitude_bounds(9).is_err());
    for (frame, epoch) in [10, 20].into_iter().enumerate() {
        let snapshot = collection.frame(0, frame).unwrap();
        assert_eq!(snapshot.sources.len(), 1);
        assert_eq!(snapshot.sources[0].capture_time, epoch);
        assert_eq!(snapshot.average_iq.len(), 7480);
        for delay in [false, true] {
            let view = collection::viewport(&snapshot, delay, 100, 500).unwrap();
            assert_eq!(view["start"], 100);
            assert_eq!(view["count"], 500);
        }
    }
    assert!(collection.frame(0, 2).is_err());
    assert!(collection.frame(9, 0).is_err());
    for delay in [false, true] {
        let view = collection::viewport(&result, delay, 100, 500).unwrap();
        assert_eq!(view["start"], 100);
        assert_eq!(view["count"], 500);
        assert_eq!(view["delay"], delay);
        assert_eq!(view["unit"], if delay { "µs" } else { "dB" });
        assert!(view["points"].as_array().unwrap().len() <= 1024);
        assert_eq!(view["points"][0]["sample_index"], 100);
        assert_eq!(
            view["points"].as_array().unwrap().last().unwrap()["sample_index"],
            599
        );
    }
    assert!(collection::viewport(&result, false, usize::MAX, 1).is_err());
    assert!(collection.aggregate(9).is_err());
    let path = dir.path().join("result.json");
    write_json_new(&result, &path).unwrap();
    assert!(write_json_new(&result, &path).is_err());
    let export: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(export["average_iq"].as_array().unwrap().len(), 7480);
    assert_eq!(export["group_delay_s"].as_array().unwrap().len(), 7480);
}
#[test]
fn archive_import_is_content_based_bounded_and_cancelable() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("captures.zip");
    let file = std::fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    for epoch in [20, 10] {
        zip.start_file(
            format!("nested/{epoch}.dat"),
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(&capture(epoch, 34)).unwrap();
    }
    zip.finish().unwrap();
    let collection =
        Collection::scan(&path, ImportLimits::default(), &AtomicBool::new(false)).unwrap();
    assert_eq!(collection.catalog.accepted_count, 2);
    assert!(Collection::scan(&path, ImportLimits::default(), &AtomicBool::new(true)).is_err());
    let limits = ImportLimits {
        max_samples: 10,
        ..ImportLimits::default()
    };
    assert!(Collection::scan(&path, limits, &AtomicBool::new(false)).is_err());
}
