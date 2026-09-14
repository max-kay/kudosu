#!/usr/bin/env bash
set -e

AVD_NAME="Pixel_API34"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
EMULATOR="$ANDROID_HOME/emulator/emulator"
ADB="$ANDROID_HOME/platform-tools/adb"
ENABLE_LOG=true
RUN_APP=false
BIAS=35

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-log)
            ENABLE_LOG=false
            shift
            ;;
        --run|-r)
            RUN_APP=true
            shift
            ;;
        --avd)
            AVD_NAME="$2"
            shift 2
            ;;
        --bias)
            BIAS="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: ./start_emulator.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --run, -r       Immediately run 'cargo apk run' after emulator setup"
            echo "  --no-log        Do not open the Logcat window"
            echo "  --avd <name>    Specify AVD name (default: Pixel_API34)"
            echo "  --bias <0-100>  Height percentage for top Logcat pane (default: 35)"
            echo "  -h, --help      Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# 1. Verify required Android tools
if [[ ! -x "$ADB" ]]; then
    echo "Error: adb not found at $ADB" >&2
    exit 1
fi
if [[ ! -x "$EMULATOR" ]]; then
    echo "Error: emulator not found at $EMULATOR" >&2
    exit 1
fi

# 2. Check if emulator or device is already running
echo "==> Checking for connected Android devices..."
DEVICE_COUNT=$("$ADB" devices | awk 'NR>1 && $2=="device" {count++} END {print count+0}')

if [[ "$DEVICE_COUNT" -eq 0 ]]; then
    echo "==> No active device found. Starting emulator '$AVD_NAME'..."
    "$EMULATOR" -avd "$AVD_NAME" -no-boot-anim -no-snapshot-load >/dev/null 2>&1 &
    EMULATOR_PID=$!
    echo "    Emulator launched in background (PID $EMULATOR_PID)."

    echo "==> Waiting for device to connect to ADB..."
    "$ADB" wait-for-device

    echo "==> Waiting for Android system boot to complete..."
    until [[ "$("$ADB" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == "1" ]]; do
        sleep 1
    done
    echo "==> Emulator is fully booted and ready!"
else
    echo "==> Active device/emulator detected."
fi

# 3. Launch Logcat in a new Kitty pane ABOVE the current one
if [[ "$ENABLE_LOG" == "true" ]]; then
    echo "==> Clearing old log buffer..."
    "$ADB" logcat -c 2>/dev/null || true

    LOGCAT_CMD="\"$ADB\" logcat -v color -s Kudosu:V TinySkiaCounter:V AndroidRuntime:E DEBUG:E"

    if command -v kitten >/dev/null 2>&1; then
        echo "==> Opening Logcat viewer in Kitty window above..."
        # Close any previously opened Logcat window in this tab
        kitten @ close-window --match "title:^Logcat$" 2>/dev/null || true

        # Switch layout to splits or vertical so splits stack top-to-bottom
        kitten @ goto-layout splits 2>/dev/null || kitten @ goto-layout vertical 2>/dev/null || true

        # Launch Logcat in a pane BEFORE (above) the current one, keeping focus in the bottom window
        if ! kitten @ launch \
            --type=window \
            --location=before \
            --keep-focus \
            --bias="$BIAS" \
            --title="Logcat" \
            --cwd="$PROJECT_DIR" \
            sh -c "$LOGCAT_CMD" 2>/dev/null; then

            # Fallback if --bias is not supported by the active layout
            kitten @ launch \
                --type=window \
                --location=before \
                --keep-focus \
                --title="Logcat" \
                --cwd="$PROJECT_DIR" \
                sh -c "$LOGCAT_CMD" 2>/dev/null || true
        fi
    elif command -v kitty >/dev/null 2>&1; then
        kitty --single-instance --title="Logcat" --directory="$PROJECT_DIR" sh -c "$LOGCAT_CMD" >/dev/null 2>&1 &
        echo "==> Opened Logcat in a separate Kitty window."
    fi
fi

# 4. Ready
echo ""
echo "============================================================"
echo "  ✓ Emulator is running & ready!"
if [[ "$ENABLE_LOG" == "true" ]]; then
    echo "  ✓ Logcat is streaming in the window above."
fi
echo "============================================================"
echo "  You can now run your app in this window:"
echo "    cargo apk run"
echo ""
echo "  To reload after code changes:"
echo "    1. Press Ctrl+C to stop the current run"
echo "    2. Run 'cargo apk run' again"
echo "============================================================"
echo ""

if [[ "$RUN_APP" == "true" ]]; then
    echo "==> Launching app via cargo apk run..."
    cd "$PROJECT_DIR"
    exec cargo apk run
fi
