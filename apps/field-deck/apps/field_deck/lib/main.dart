// Field Deck: phone supervisor over the live daemon API.
// Supervisor-only by rule: watch the board, read diffs, queue
// approvals, view audit. No on-phone agent execution, ever.
import 'package:flutter/material.dart';
import 'package:harness_ui/harness_ui.dart';
import 'screens/audit_screen.dart';
import 'screens/board_screen.dart';
import 'screens/review_screen.dart';
import 'screens/settings_screen.dart';
import 'screens/worker_screen.dart';
import 'session.dart';

void main() => runApp(const FieldDeckApp());

class FieldDeckApp extends StatelessWidget {
  const FieldDeckApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Field Deck',
      theme: HxTheme.light(),
      darkTheme: HxTheme.dark(),
      home: const HxShell(),
    );
  }
}

class HxShell extends StatefulWidget {
  const HxShell({super.key});

  @override
  State<HxShell> createState() => _HxShellState();
}

class _HxShellState extends State<HxShell> {
  int _tab = 0;
  HxConnection? _connection;
  final HxIntentQueue _queue = HxIntentQueue();

  @override
  void initState() {
    super.initState();
    _boot();
  }

  Future<void> _boot() async {
    final connection = await loadConnection();
    final saved = await loadQueue();
    if (!mounted) return;
    setState(() {
      _connection = connection;
      for (final i in saved.pending) {
        _queue.enqueue(i);
      }
    });
  }

  void _enqueue(HxIntent intent) {
    _queue.enqueue(intent);
    // ignore: unawaited_futures (persist in background; queue is in memory)
    saveQueue(_queue);
    setState(() {});
  }

  Future<void> _savePairing(String host, String bearer) async {
    await saveConnection(host, bearer);
    if (!mounted) return;
    setState(() => _connection = HxConnection(baseUrl: host, bearer: bearer));
  }

  @override
  Widget build(BuildContext context) {
    final connection = _connection;
    if (connection == null) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }
    final api = connection.api;
    final screens = [
      HxBoardScreen(api: api),
      HxWorkerScreen(api: api),
      HxReviewScreen(api: api, onQueue: _enqueue),
      HxAuditScreen(api: api),
      HxSettingsScreen(
        connection: connection,
        queued: _queue.length,
        // ignore: unawaited_futures (pairing save completes in background)
        onSaved: (host, bearer) => _savePairing(host, bearer),
      ),
    ];
    return Scaffold(
      appBar: AppBar(title: const Text('Field Deck')),
      body: IndexedStack(index: _tab, children: screens),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _tab,
        onDestinationSelected: (i) => setState(() => _tab = i),
        destinations: const [
          NavigationDestination(
            icon: Icon(Icons.dashboard_outlined),
            label: 'Board',
          ),
          NavigationDestination(
            icon: Icon(Icons.smart_toy_outlined),
            label: 'Worker',
          ),
          NavigationDestination(
            icon: Icon(Icons.rate_review_outlined),
            label: 'Review',
          ),
          NavigationDestination(
            icon: Icon(Icons.fact_check_outlined),
            label: 'Audit',
          ),
          NavigationDestination(
            icon: Icon(Icons.settings_outlined),
            label: 'Settings',
          ),
        ],
      ),
    );
  }
}
