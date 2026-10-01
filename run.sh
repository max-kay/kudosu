CARGO_APK_RELEASE_KEYSTORE="$HOME/.android/debug.keystore" \
CARGO_APK_RELEASE_KEYSTORE_PASSWORD="android" \
CARGO_APK_RELEASE_KEY_ALIAS="androiddebugkey" \
CARGO_APK_RELEASE_KEY_PASSWORD="android" \
cargo apk run --release
