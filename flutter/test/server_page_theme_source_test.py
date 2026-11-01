"""Narrow source guard, runnable without Flutter/native bindings."""
from pathlib import Path
import unittest

SOURCE = (Path(__file__).resolve().parents[1] / 'lib/desktop/pages/server_page.dart').read_text()


class ServerPageThemeSourceTest(unittest.TestCase):
    def test_acceptance_uses_paired_semantic_colors(self):
        panel = SOURCE.split('class _CmControlPanel', 1)[1].split('  Widget buildButton', 1)[0]
        self.assertNotIn('Colors.white', panel)
        self.assertNotIn('MyTheme.accent', panel)
        self.assertIn('colors.acceptBackground', panel)
        self.assertIn('colors.acceptForeground', panel)
        self.assertIn('colors.cancelForeground', panel)

    def test_elevation_buttons_use_distinct_elevate_pair(self):
        panel = SOURCE.split('class _CmControlPanel', 1)[1].split('  Widget buildButton', 1)[0]
        for label in ("text: 'Accept and Elevate'", "text: 'Elevate'"):
            start = panel.index(label)
            block = panel[max(0, start - 500):start + 300]
            self.assertIn('colors.elevateBackground', block)
            self.assertIn('colors.elevateForeground', block)
        accept = panel.index("text: 'Accept',")
        self.assertIn('colors.acceptBackground', panel[accept - 300:accept])

    def test_permission_camera_icon_uses_semantic_state_pair(self):
        board = SOURCE.split('class _PrivilegeBoardState', 1)[1].split('const double buttonBottomMargin', 1)[0]
        self.assertNotIn('Colors.white', board)
        self.assertNotIn('Colors.grey[700]', board)
        self.assertIn('colors.permissionBackground(enabled, canModify)', board)
        self.assertIn('colors.permissionForeground(enabled, canModify)', board)
        self.assertIn('Icons.videocam_rounded', board)

    def test_header_svg_has_explicit_semantic_filter(self):
        header = SOURCE.split('class _CmHeaderState', 1)[1].split('class _PrivilegeBoard', 1)[0]
        self.assertRegex(header, r'colorFilter:\s*ColorFilter\.mode\(')
        self.assertIn('colors.headerForeground, BlendMode.srcIn', header)
        self.assertNotIn('Colors.white', header)


if __name__ == '__main__':
    unittest.main()
