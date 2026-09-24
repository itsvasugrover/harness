// Worker tab: status, last-known facts, and detail dialogs.
// Reads board facts only; execution stays on the daemon by rule.
import 'package:flutter/material.dart';
import 'package:harness_ui/harness_ui.dart';

class HxWorkerScreen extends StatefulWidget {
  final HxApi api;
  const HxWorkerScreen({super.key, required this.api});

  @override
  State<HxWorkerScreen> createState() => _HxWorkerScreenState();
}

class _HxWorkerScreenState extends State<HxWorkerScreen> {
  late Future<HxBoard> _future;

  @override
  void initState() {
    super.initState();
    _future = widget.api.board();
  }

  Future<void> _refresh() async {
    final board = await widget.api.board();
    if (mounted) setState(() => _future = Future.value(board));
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<HxBoard>(
      future: _future,
      builder: (context, snap) {
        if (snap.connectionState == ConnectionState.waiting) {
          return const Center(child: CircularProgressIndicator());
        }
        if (snap.hasError || !snap.hasData || snap.data!.workers.isEmpty) {
          return const HxEmpty(
            icon: Icons.smart_toy_outlined,
            title: 'No workers',
            hint: 'Workers appear once a desktop goal is running.',
          );
        }
        final workers = snap.data!.workers;
        return RefreshIndicator(
          onRefresh: _refresh,
          child: ListView(
            children: [
              for (final w in workers)
                HxCard(
                  onTap: () => showDialog<void>(
                    context: context,
                    builder: (_) => AlertDialog(
                      title: Text(
                        w.workerId,
                        style: Theme.of(context).textTheme.titleSmall,
                      ),
                      content: Text(
                        'Status: ${w.column}\n'
                        'Liveness: ${w.alive ? 'alive' : 'stopped'}\n'
                        'Blocker: ${w.blocked ?? 'none'}\n'
                        'Completed: ${w.completed ? 'yes' : 'no'}',
                      ),
                      actions: [
                        TextButton(
                          onPressed: () => Navigator.of(context).pop(),
                          child: const Text('Close'),
                        ),
                      ],
                    ),
                  ),
                  child: Row(
                    children: [
                      Icon(
                        w.completed
                            ? Icons.check_circle_outline
                            : Icons.smart_toy_outlined,
                        color: context.hx.muted,
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            HxText(w.workerId, role: HxTextRole.mono),
                            HxText(
                              w.completed
                                  ? 'finished'
                                  : (w.blocked ?? 'running'),
                              role: HxTextRole.caption,
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
            ],
          ),
        );
      },
    );
  }
}
