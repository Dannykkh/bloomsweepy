fn main() {
    #[cfg(windows)]
    {
        let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
        let attributes = tauri_build::Attributes::new().windows_attributes(windows);
        tauri_build::try_build(attributes).expect("failed to build the Windows app resources");
        embed_resource::compile_for_everything("windows-test-manifest.rc", embed_resource::NONE)
            .manifest_required()
            .expect("failed to embed the Windows common-controls manifest");
    }

    #[cfg(not(windows))]
    tauri_build::build();
}
