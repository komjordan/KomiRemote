import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


def read(rel):
    return (ROOT / rel).read_text(encoding="utf-8")


class KomiV14(unittest.TestCase):
    def test_proto_carries_controller_name(self):
        proto = read("libs/base/protos/message.proto")
        block = proto[proto.index("message CursorPosition") :]
        block = block[: block.index("}")]
        self.assertIn("string controller_name = 3;", block)

    def test_host_sets_and_clears_controller(self):
        conn = read("src/server/connection.rs")
        self.assertIn("note_peer_cursor_controller(self.inner.id(), &self.lr.my_name)", conn)
        self.assertIn("clear_peer_cursor_controller(self.0)", conn)
        inp = read("src/server/input_service.rs")
        self.assertIn("controller_name", inp)
        self.assertIn("moving_name", inp)
        self.assertIn("named_target", inp)

    def test_client_forwards_sanitized_name(self):
        src = read("src/flutter.rs")
        self.assertIn("sanitize_controller_name(&cp.controller_name)", src)

    def test_name_tag_overlay_ignores_pointer(self):
        tag = read("flutter/lib/common/widgets/peer_cursor_name_tag.dart")
        self.assertIn("IgnorePointer(", tag)
        page = read("flutter/lib/desktop/pages/remote_page.dart")
        self.assertIn("const PeerCursorNameTagPaint()", page)
        model = read("flutter/lib/models/model.dart")
        self.assertIn("_setPeerControllerName(evt['controller_name'])", model)
        self.assertIn('"controller_name"', read("src/flutter.rs"))

    def test_display_name_priority(self):
        client = read("src/client.rs")
        self.assertIn("OPTION_KOMI_DISPLAY_NAME", client)
        self.assertIn("select_display_name(", client)
        rs = read("src/komi_cursor_tag.rs")
        self.assertIn('"komi-display-name"', rs)
        dart = read("flutter/lib/desktop/komi_public_names.dart")
        self.assertIn("kKomiDisplayNameOption = 'komi-display-name'", dart)
        home = read("flutter/lib/desktop/pages/desktop_home_page.dart")
        self.assertIn("_syncSessionDisplayName(komiActivePublicName.value)", home)
        self.assertIn("_syncSessionDisplayName(confirmed)", home)

    def test_remote_cursor_default_on(self):
        cfg = read("src/komi_config.rs")
        self.assertRegex(cfg, r'\("show_remote_cursor",\s*"Y"\)')

    def test_toolbar_button_sizes(self):
        compact = read("flutter/lib/desktop/widgets/remote_toolbar_compact.dart")
        self.assertIn("static const double hitTargetSize = 32", compact)
        bar = read("flutter/lib/desktop/widgets/remote_toolbar.dart")
        self.assertIn("static const double buttonHMargin = 2;", bar)
        self.assertIn("static const double buttonVMargin = 6;", bar)
        self.assertIn("_ToolbarTheme.buttonMargin(widget.isHorizontal)", bar)
        self.assertIn("barBackground(context)", bar)

    def test_build_number(self):
        # v14 shipped as 1.5.1+69; later builds keep 1.5.1 and raise the build.
        m = re.search(r"(?m)^version: 1\.5\.1\+(\d+)$", read("flutter/pubspec.yaml"))
        self.assertIsNotNone(m)
        self.assertGreaterEqual(int(m.group(1)), 69)


if __name__ == "__main__":
    unittest.main()
