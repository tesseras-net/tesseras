import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../providers/theme_provider.dart';
import '../widgets/sidebar.dart';
import 'timeline/timeline_screen.dart';
import 'network/network_screen.dart';
import 'settings/settings_screen.dart';
import 'create_memory/create_memory_dialog.dart';

class DesktopShell extends ConsumerStatefulWidget {
  const DesktopShell({super.key});

  @override
  ConsumerState<DesktopShell> createState() => _DesktopShellState();
}

class _DesktopShellState extends ConsumerState<DesktopShell> {
  int _currentIndex = 0;
  final FocusNode _searchFocusNode = FocusNode();

  @override
  void dispose() {
    _searchFocusNode.dispose();
    super.dispose();
  }

  void _showCreateMemoryDialog() {
    showDialog(
      context: context,
      builder: (context) => const CreateMemoryDialog(),
    );
  }

  @override
  Widget build(BuildContext context) {
    final screens = <Widget>[
      TimelineScreen(searchFocusNode: _searchFocusNode),
      const NetworkScreen(),
      const SettingsScreen(),
    ];

    return Shortcuts(
      shortcuts: <ShortcutActivator, Intent>{
        LogicalKeySet(LogicalKeyboardKey.control, LogicalKeyboardKey.keyN):
            const _CreateMemoryIntent(),
        LogicalKeySet(LogicalKeyboardKey.control, LogicalKeyboardKey.digit1):
            const _SwitchTabIntent(0),
        LogicalKeySet(LogicalKeyboardKey.control, LogicalKeyboardKey.digit2):
            const _SwitchTabIntent(1),
        LogicalKeySet(LogicalKeyboardKey.control, LogicalKeyboardKey.digit3):
            const _SwitchTabIntent(2),
        LogicalKeySet(LogicalKeyboardKey.control, LogicalKeyboardKey.keyF):
            const _FocusSearchIntent(),
        LogicalKeySet(LogicalKeyboardKey.control, LogicalKeyboardKey.keyD):
            const _ToggleThemeIntent(),
      },
      child: Actions(
        actions: <Type, Action<Intent>>{
          _CreateMemoryIntent:
              CallbackAction<_CreateMemoryIntent>(onInvoke: (_) {
            _showCreateMemoryDialog();
            return null;
          }),
          _SwitchTabIntent: CallbackAction<_SwitchTabIntent>(onInvoke: (intent) {
            setState(() => _currentIndex = intent.index);
            return null;
          }),
          _FocusSearchIntent:
              CallbackAction<_FocusSearchIntent>(onInvoke: (_) {
            if (_currentIndex != 0) {
              setState(() => _currentIndex = 0);
            }
            _searchFocusNode.requestFocus();
            return null;
          }),
          _ToggleThemeIntent:
              CallbackAction<_ToggleThemeIntent>(onInvoke: (_) {
            ref.read(themeProvider.notifier).toggle();
            return null;
          }),
        },
        child: Focus(
          autofocus: true,
          child: Scaffold(
            child: Row(
              children: [
                Sidebar(
                  selectedIndex: _currentIndex,
                  onItemSelected: (index) =>
                      setState(() => _currentIndex = index),
                  onCreateMemory: _showCreateMemoryDialog,
                ),
                Expanded(child: screens[_currentIndex]),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _CreateMemoryIntent extends Intent {
  const _CreateMemoryIntent();
}

class _SwitchTabIntent extends Intent {
  final int index;
  const _SwitchTabIntent(this.index);
}

class _FocusSearchIntent extends Intent {
  const _FocusSearchIntent();
}

class _ToggleThemeIntent extends Intent {
  const _ToggleThemeIntent();
}
