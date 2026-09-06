# Documentation screenshots

The `v1.6.0-*` images are browser captures of the current app's actual React views, not generated artwork. They use only synthetic drives, filenames, memory/CPU values, process IDs, and sample conversations. They are not evidence of real AI answers or native backend execution. Browser rendering does not reproduce macOS native window vibrancy.

## Reproduce

1. From `apps/desktop`, run `npm ci` and `npm run dev`.
2. Open `http://127.0.0.1:1420/release-preview.html?view=dashboard&language=en`.
3. Set a 1440 × 960 CSS-pixel viewport. Wait for the view, fonts, and initial metric sample to settle before capture.
4. Capture PNG at CSS-pixel scale. Do not retouch the UI or insert real user data.

| Query | Values |
|---|---|
| `view` | `dashboard`, `overview`, `performance`, `assistant`, `settings` |
| `language` | `en`, `ko`, `ja`, `zh-CN` |

Name files `v1.6.0-{view}-{language}.png`. All four root READMEs show the five matching-language views. Include the synthetic-data notice whenever reusing these images.

The preview is a separate Vite development entry point. It is not linked from the app or included in the production build. Native IPC is mocked, and unimplemented actions fail instead of executing a CLI, changing files, requesting process termination, or changing OS settings. The existing broom icon remains the application identity; it was not replaced with generated artwork.

Older versioned images remain for historical documentation. Local QA screenshots and original video-production assets are not release screenshots.
