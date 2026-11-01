import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class KomiDeviceNameIntegration(unittest.TestCase):
    def test_uses_dedicated_persistent_config_key_and_native_hostname(self):
        value = (ROOT / "src/komi_device_name.rs").read_text(encoding="utf-8")
        connection = (ROOT / "src/server/connection.rs").read_text(encoding="utf-8")
        home = (ROOT / "flutter/lib/desktop/pages/desktop_home_page.dart").read_text(
            encoding="utf-8"
        )
        self.assertIn('"komi-device-name"', value)
        self.assertIn("Config::get_option", connection)
        self.assertIn("pi.hostname =", connection)
        self.assertIn("mainSetOption", home)
        self.assertIn("mainGetOptionSync", home)

    def test_does_not_change_id_or_protocol_schema(self):
        connection = (ROOT / "src/server/connection.rs").read_text(encoding="utf-8")
        helper = (ROOT / "src/komi_device_name.rs").read_text(encoding="utf-8")
        self.assertNotIn("set_id", helper)
        self.assertNotIn("set_id", connection[connection.index("OPTION_KOMI_DEVICE_NAME") - 300 : connection.index("OPTION_KOMI_DEVICE_NAME") + 300])
        proto = (ROOT / "libs/base/protos/message.proto").read_text(encoding="utf-8")
        self.assertIn("string hostname = 2;", proto)

    def test_recent_priority_keeps_manual_alias_first_and_id_visible(self):
        helper = (ROOT / "flutter/lib/desktop/komi_device_name.dart").read_text(
            encoding="utf-8"
        )
        card = (ROOT / "flutter/lib/common/widgets/peer_card.dart").read_text(
            encoding="utf-8"
        )
        self.assertLess(helper.index("cleanAlias.isNotEmpty"), helper.index("cleanHostname.isNotEmpty"))
        self.assertIn("widget.tab == PeerTabIndex.recent", card)
        self.assertIn("formatID(peer.id)", card)
        self.assertIn("mainSetPeerAlias", card)


if __name__ == "__main__":
    unittest.main()
