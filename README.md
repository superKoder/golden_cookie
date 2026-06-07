# Never miss another "golden cookie"!

This is a tool for playing **Ortel's [Cookie Clicker](http://cookieclicker.com/)**. In that game, a ***"golden cookie"*** ramndomly appears and needs to be clicked within seconds.

This is a lightweight, efficient, cross-platform background clicker that captures the screen every 2 seconds, detects the "golden cookie" using **Zero-Mean Normalized Cross-Correlation (ZNCC)** with alpha-channel masking (ignoring changing backgrounds), and clicks it!

The app contains the cookie image embedded directly inside it, so it is fully self-contained!

---

## macOS `.app` Bundle

### How to Build & Package

We have provided a packaging script `package_mac.sh` which compiles the code and wraps it in a `.app` bundle:

```shell
./package_mac.sh
```

This generates **`GoldenCookie.app`** in the project directory.

### How to Run

1. Double-click **`GoldenCookie.app`** in Finder, or run:

   ```shell
   open GoldenCookie.app
   ```

2. **System Prompts:** Upon launch, macOS will prompt you to grant:
   - **`Screen Recording`** permission to **`goldeCookie.app`**.
   - **`Accessibility`** permission to **`goldenCookie.app`**.

3. Toggle both ON in `System Settings` > `Privacy & Security`.

### How to Stop

- **Auto-quit:** Move your mouse cursor to any of the 4 absolute corners of your screen (top-left, top-right, bottom-left, or bottom-right). This triggers a built-in fail-safe that exits the app immediately.

- **Terminal command:** Run `pkill goldenCookie` in your terminal.

---

## Command Line Interface (CLI) on any platform

### Running the binary itself

If you still want to run the raw binary. For example, for development or debugging:

```shell
./target/release/golden_cookie
```

NOTE: On macOS, doing so equires *Screen Recording* and *Accessibility* permissions granted to the Terminal / IDE running the command. So it is smarted to run the `.app` version above.

### Testing itsef

To test detection on your current screen and save a diagnostic image (`match_result_<Monitor>.png`) showing where the cookie was found (Green box for match, Blue box for best guess):

```shell
./target/release/golden_cookie --test
```
