// Settings tab: host pairing, queued-intent count, theme note.
// Host discovery runs through the unauthenticated identity probe so
// the host id is visible before any token is pasted.
import 'package:flutter/material.dart';
import 'package:harness_ui/harness_ui.dart';
import '../session.dart';

class HxSettingsScreen extends StatefulWidget {
  final HxConnection connection;
  final int queued;
  final void Function(String host, String bearer) onSaved;
  const HxSettingsScreen({
    super.key,
    required this.connection,
    required this.queued,
    required this.onSaved,
  });

  @override
  State<HxSettingsScreen> createState() => _HxSettingsScreenState();
}

class _HxSettingsScreenState extends State<HxSettingsScreen> {
  late final TextEditingController _host;
  late final TextEditingController _bearer;
  String? _probe;

  @override
  void initState() {
    super.initState();
    _host = TextEditingController(text: widget.connection.baseUrl);
    _bearer = TextEditingController(text: widget.connection.bearer);
  }

  @override
  void dispose() {
    _host.dispose();
    _bearer.dispose();
    super.dispose();
  }

  Future<void> _test() async {
    setState(() => _probe = 'probing…');
    try {
      final id = await HxApi(baseUrl: _host.text.trim()).identity();
      final mismatch = id.contract > hxContract ? ' (newer than phone!)' : '';
      if (mounted) {
        setState(
          () => _probe = '${id.hostId} · contract ${id.contract}$mismatch',
        );
      }
    } catch (e) {
      if (mounted) setState(() => _probe = 'unreachable: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const HxText('Pairing', role: HxTextRole.title),
        const SizedBox(height: 8),
        TextField(
          controller: _host,
          decoration: const InputDecoration(
            labelText: 'Daemon URL',
            hintText: 'http://192.168.1.10:4317',
            border: OutlineInputBorder(),
          ),
          keyboardType: TextInputType.url,
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _bearer,
          decoration: const InputDecoration(
            labelText: 'Bearer token',
            border: OutlineInputBorder(),
          ),
          obscureText: true,
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            FilledButton(
              onPressed: () {
                widget.onSaved(_host.text.trim(), _bearer.text.trim());
                ScaffoldMessenger.of(
                  context,
                ).showSnackBar(const SnackBar(content: Text('Pairing saved')));
              },
              child: const Text('Save'),
            ),
            const SizedBox(width: 8),
            OutlinedButton(onPressed: _test, child: const Text('Test')),
          ],
        ),
        if (_probe != null) ...[
          const SizedBox(height: 8),
          HxText(_probe!, role: HxTextRole.mono),
        ],
        const SizedBox(height: 24),
        const HxText('Offline', role: HxTextRole.title),
        const SizedBox(height: 8),
        HxText(
          '${widget.queued} intent(s) queued — replay on reconnect.',
          role: HxTextRole.body,
        ),
      ],
    );
  }
}
