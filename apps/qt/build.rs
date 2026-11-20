/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
use cxx_qt_build::{CxxQtBuilder, QmlModule};

/// Compiles the branded Windows shell icon into the GUI executable.
///
/// Uses the active Windows SDK resource compiler without changing PATH or
/// adding a runtime dependency. The resource is linked only into the GUI binary.
///
/// # Panics
/// Terminates the build if required paths or the resource compiler are missing.
fn windows_icon() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.parent().and_then(std::path::Path::parent).unwrap();
    let icon = root.join("packaging/windows/raven.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let source = output.join("viewer-icon.rc");
    let resource = output.join("viewer-icon.res");
    let icon_path = icon
        .to_str()
        .expect("Icon path must be UTF-8")
        .replace('\\', "/");
    std::fs::write(
        &source,
        format!("#pragma code_page(65001)\n1 ICON \"{icon_path}\"\n"),
    )
    .unwrap();
    let status = std::process::Command::new("rc.exe")
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource)
        .arg(&source)
        .status()
        .expect("Windows SDK rc.exe is required to embed the application icon");
    assert!(status.success(), "Windows icon resource compilation failed");
    println!("cargo:rustc-link-arg-bin=raven={}", resource.display());
}
/// Compiles branding resources while tracking only real source paths.
///
/// The pinned qt-build-utils resource-list helper includes its trailing blank
/// line. Passing that empty path to Cargo watches the entire crate, including
/// generated outputs, and repeatedly invalidates otherwise unchanged builds.
/// Keep the same rcc compiler and resource aliases without modifying the SDK.
///
/// # Returns
/// Generated C++ resource source, whose initializer must be linked directly.
///
/// # Panics
/// Stops the build if Qt tools or any resource input cannot be read/compiled.
fn branding_resources() -> std::path::PathBuf {
    let mut qt = qt_build_utils::QtBuild::new(vec!["Core".into()])
        .expect("Could not find Qt for branding resources");
    println!("cargo::rerun-if-changed=branding.qrc");
    println!("cargo::rerun-if-changed=src/branding.h");
    for resource in qt.qrc_list(&"branding.qrc") {
        if !resource.as_os_str().is_empty() {
            println!("cargo::rerun-if-changed={}", resource.display());
        }
    }
    qt.qrc(&"branding.qrc")
}

/// Compiles the Qt bridge and embeds the local QML interface.
///
/// Links the system Qt Quick renderer; no WebView assets are included.
fn main() {
    windows_icon();
    let branding = branding_resources();
    CxxQtBuilder::new()
        .cc_builder(|builder| {
            let mut resource_builder = builder.clone();
            resource_builder.cpp(true).std("c++17").file(&branding);

            // Link the object directly so its resource initializer runs before
            // QML loads images, even when no Rust symbol references the object.
            for object in resource_builder.compile_intermediates() {
                println!("cargo::rustc-link-arg={}", object.display());
            }
            builder.include("src");
        })
        .qt_module("Network")
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .qml_module(QmlModule {
            uri: "Pnm.Viewer",
            rust_files: &[
                "src/ai.rs",
                "src/reports.rs",
                "src/backend.rs",
                "src/update.rs",
                "src/spectrum_collection.rs",
                "src/channel_estimation_collection.rs",
                "src/profile_performance.rs",
            ],
            qml_files: &[
                "qml/Main.qml",
                "qml/AiAssistanceChip.qml",
                "qml/AiServicesDialog.qml",
                "qml/ai/connection/AiProviderSettingsDialog.qml",
                "qml/AiAnalysisDialog.qml",
                "qml/AiTextPane.qml",
                "qml/ai/report/AiReportView.qml",
                "qml/ai/report/ReportBlocks.qml",
                "qml/ai/report/ReportMarkdown.qml",
                "qml/ai/report/ReportChart.qml",
                "qml/ai/report/ReportTable.qml",
                "qml/ai/report/ReportImage.qml",
                "qml/ai/report/ReportEvidence.qml",
                "qml/ai/report/ReportExpandable.qml",
                "qml/ai/report/ReportUnavailable.qml",
                "qml/ai/report/EngineeringReportDialog.qml",
                "qml/ai/report/ReportSmoke.qml",
                "qml/AiSetupSmoke.qml",
                "qml/AboutDialog.qml",
                "qml/UpdateChip.qml",
                "qml/Theme.qml",
                "qml/ThemePalette.qml",
                "qml/ThemeTextArea.qml",
                "qml/ThemeTextField.qml",
                "qml/ThemeComboBox.qml",
                "qml/ChartSettingsDialog.qml",
                "qml/ThemeButton.qml",
                "qml/ThemeCheckBox.qml",
                "qml/PreEqComparisonSidebar.qml",
                "qml/FileErrorDialog.qml",
                "qml/RxMerCollectionView.qml",
                "qml/RxMerPlaybackControls.qml",
                "qml/RxMerCollectionTree.qml",
                "qml/MetadataChip.qml",
                "qml/PreEqComparisonChart.qml",
                "qml/PreEqComparisonView.qml",
                "qml/MeasurementChart.qml",
                "qml/SpectrumControls.qml",
                "qml/SpectrumCollectionView.qml",
                "qml/ChannelEstimationCollectionView.qml",
                "qml/ChannelEstimationCollectionSmoke.qml",
                "qml/PreEqWorkspace.qml",
                "qml/OfdmProfileWorkspace.qml",
                "qml/OfdmProfileSmoke.qml",
                "qml/OfdmProfileInputDialog.qml",
                "qml/OfdmFecResultsDialog.qml",
                "qml/SpectrumTools.qml",
                "qml/SpectrumPowerChip.qml",
                "qml/IqScatterChart.qml",
                "qml/RxMerBitLoadingDialog.qml",
                "qml/RxMerBitLoadingChart.qml",
                "qml/ConstellationChart.qml",
                "qml/ConstellationMerOverlay.qml",
                "qml/HistogramChart.qml",
                "qml/FecSummaryChart.qml",
                "qml/PanelToggle.qml",
                "qml/SmokeTest.qml",
                "qml/CaptureDropArea.qml",
                "qml/ModulationControls.qml",
                "qml/MeasurementToggle.qml",
                "qml/ProfileBandwidth.qml",
            ],
            ..Default::default()
        })
        .build();
}
