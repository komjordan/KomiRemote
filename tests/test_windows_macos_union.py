"""Source seam checks, not compilation or device tests."""
from pathlib import Path
import re
import unittest
ROOT = Path(__file__).resolve().parents[1]

class DesktopUnion(unittest.TestCase):
    def test_both_desktop_media_paths_preserve_windows_features(self):
        source = lambda p: (ROOT / p).read_text()
        lib = source('src/lib.rs')
        self.assertIn('mod komi_pills;', lib)
        self.assertIn('pub mod komi_host_priority;', lib)
        self.assertIn('mod komi_video_call;', lib)
        video = source('src/komi_video_call.rs')
        self.assertIn('scrap::PixelBuffer::from_camera_rgba', video)
        self.assertIn('scrap::PixelBuffer::new', video)
        self.assertIn('komi_host_priority = 1002', source('libs/base/protos/message.proto'))
        self.assertIn('video_accepted_at: c.komi_video_accepted_at', source('src/ui_cm_interface.rs'))
        self.assertIn('komi_peer_file', source('src/flutter_ffi.rs'))
        self.assertIn("peer['komi_peer_file'] == true", source('flutter/lib/common.dart'))
        self.assertIn('requestRecordCamera', source('flutter/macos/Runner/MainFlutterWindow.swift'))
        self.assertIn('input-avfoundation', source('libs/scrap/Cargo.toml'))
        self.assertIn('"controller_name"', source('src/flutter.rs'))
        self.assertIn("evt['controller_name']", source('flutter/lib/models/model.dart'))
        connection = source('src/server/connection.rs')
        self.assertRegex(connection, r'#\[cfg\(target_os = "windows"\)\]\s*if self.view_camera && komi_take_camera_share')
        self.assertNotIn('Windows uniquement', source('flutter/lib/desktop/komi_public_names.dart'))
        self.assertIn('KEYCHAIN_SERVICE', source('src/komi_names.rs'))
        self.assertIn('CryptProtectData', source('src/komi_names.rs'))
        self.assertIn('version: 1.5.1+85', source('flutter/pubspec.yaml'))
        self.assertIn('kKomiCurrentBuild = 85', source('flutter/lib/desktop/komi_update_logic.dart'))

if __name__ == '__main__':
    unittest.main()
