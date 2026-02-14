import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'app.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();

  runApp(const ProviderScope(child: _LifecycleManager(child: TesserasApp())));
}

/// Observes app lifecycle to manage the embedded node.
class _LifecycleManager extends StatefulWidget {
  final Widget child;
  const _LifecycleManager({required this.child});

  @override
  State<_LifecycleManager> createState() => _LifecycleManagerState();
}

class _LifecycleManagerState extends State<_LifecycleManager>
    with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // The nodeProvider handles start/stop via Riverpod lifecycle.
    // This observer is a hook point for future background sync logic.
    if (state == AppLifecycleState.paused) {
      debugPrint('Tesseras: app paused');
    }
    if (state == AppLifecycleState.resumed) {
      debugPrint('Tesseras: app resumed');
    }
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
