#!/bin/bash
# The Google Play screenshots (1080×2160), from a debug build on an emulator, in English and
# Chinese, into google-play/screenshots/{en,zh}/. Then: node render.mjs
#
#   ANDROID_SERIAL=emulator-5554 store/capture-android.sh
#
# The emulator must already have the debug APK installed (./gradlew :app:installDebug). It is
# taken offline and its status bar set to demo mode; `adb shell wm size reset` undoes the size.
set -euo pipefail
STORE=$(cd "$(dirname "$0")" && pwd)
REQUEST=$(sed -n 's/.*static let request = #"\(.*\)"#/\1/p' "$STORE/../app-ios/AirkeysUITests/FlowTests.swift")
UI=$(mktemp)

texts() { adb shell uiautomator dump /sdcard/ui.xml >/dev/null 2>&1; adb shell cat /sdcard/ui.xml > "$UI"; }
waitfor() {
	for _ in $(seq 40); do texts; grep -qF -- "text=\"$1\"" "$UI" && return 0; sleep 0.5; done
	echo "not on screen: $1" >&2; exit 1
}
tap() {
	waitfor "$1"
	adb shell input tap $(python3 - "$UI" "$1" <<'PY'
import re, sys
for node in re.findall(r'<node [^>]*>', open(sys.argv[1]).read()):
    if re.search(r'text="([^"]*)"', node).group(1) == sys.argv[2]:
        b = list(map(int, re.findall(r'\d+', re.search(r'bounds="([^"]*)"', node).group(1))))
        print((b[0] + b[2]) // 2, (b[1] + b[3]) // 2); break
PY
)
}
shot() { sleep 1.2; adb exec-out screencap -p > "$1"; echo "${1#$STORE/}"; }

adb shell wm size 1080x2160
adb shell cmd connectivity airplane-mode enable
adb shell svc wifi disable
adb shell cmd bluetooth_manager disable
adb shell settings put global wifi_scan_always_enabled 0
adb shell pm grant app.airkeys android.permission.CAMERA
adb shell settings put global sysui_demo_allowed 1
demo() { adb shell am broadcast -a com.android.systemui.demo -e command "$@" >/dev/null; }
demo enter
demo clock -e hhmm 0941
demo battery -e level 100 -e plugged false
demo notifications -e visible false
demo network -e airplane show -e wifi hide -e mobile hide

for lang in en zh; do
	if [ $lang = zh ]; then
		locale=zh-CN; L=(输入助记词 继续 打开钱包 扫描请求 连接 显示动态码 给网页助手扫描 完成)
	else
		locale=en-US; L=("Enter recovery phrase" Continue "Open wallet" "Scan request" Connect "Show code" "Show this to the web app" Done)
	fi
	out="$STORE/google-play/screenshots/$lang"
	mkdir -p "$out"
	adb shell am force-stop app.airkeys
	adb shell cmd locale set-app-locales app.airkeys --locales $locale
	adb shell am start -n app.airkeys/.MainActivity >/dev/null
	waitfor "${L[0]}"; shot "$out/01-welcome.png"
	tap "${L[0]}"
	sleep 1
	# four letters and a space per word (%s is a space to `input text`); the app completes them
	for _ in $(seq 11); do adb shell input text "aban%s"; done
	adb shell input text about
	sleep 1
	if adb shell dumpsys input_method | grep -q mInputShown=true; then adb shell input keyevent KEYCODE_BACK; fi
	tap "${L[1]}"; tap "${L[2]}"
	waitfor "${L[3]}"; shot "$out/02-home.png"
	tap "${L[4]}"; tap "${L[5]}"
	waitfor "${L[6]}"; shot "$out/04-code.png"
	tap "${L[7]}"; tap "${L[3]}"
	sleep 1.5
	# the debug hook: the request, as if the camera had read it
	adb shell am start -n app.airkeys/.MainActivity --es airkeys.deliver "$(printf '%s' "$REQUEST" | base64 | tr -d '\n')" >/dev/null 2>&1
	waitfor "100 USDC"; shot "$out/03-review.png"
done
demo exit
rm -f "$UI"
