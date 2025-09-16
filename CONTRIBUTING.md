**`.github/CONTRIBUTING.md`**

    ```markdown
    # How to Contribute to Intelexta

    We're excited you're here to help build a new foundation for trustworthy AI work.

    ## Development Setup

    1.  Ensure you have Rust and Node.js installed.
    2.  Follow the steps in the `README.md` to get the application running.
    3.  On some Linux systems using Wayland, the app may fail to launch due to graphics permissions. The current workaround is to run the backend with:
        `LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 WINIT_UNIX_BACKEND=x11 GDK_BACKEND=x11 cargo tauri dev`

    ## Pull Request Process

    1.  Fork the repository and create your branch from `main`.
    2.  Make sure your code lints and any new features have tests.
    3.  Submit your pull request with a clear description of the changes.
    ```
