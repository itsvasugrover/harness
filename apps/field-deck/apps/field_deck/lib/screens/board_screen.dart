// Board tab: derived Kanban over the live event stream.
// Initial authoritative fetch, then stream frames trigger refetches;
// the cursor persists so reconnects resume instead of restarting.
// Stream loss surfaces the cached board with a manual retry.
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:harness_ui/harness_ui.dart';

import '../session.dart' show loadCursor, saveCursor;

const _columns = [
  'working',
  'needs_you',
  'in_review',
  'ready_to_merge',
  'done'
];

String _label(String column) => switch (column) {
      'working' => 'Working',
      'needs_you' => 'Needs you',
      'in_review' => 'In review',
      'ready_to_merge' => 'Ready',
      _ => 'Done',
    };

class HxBoardScreen extends StatefulWidget {
  final HxApi api;
  const HxBoardScreen({super.key, required this.api});

  @override
  State<HxBoardScreen> createState() => _HxBoardScreenState();
}

class _HxBoardScreenState extends State<HxBoardScreen> {
  HxBoard? _board;
  String? _error;
  StreamSubscription<HxServerEvent>? _sub;

  @override
  void initState() {
    super.initState();
    _boot();
  }

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  Future<void> _boot() async {
    final cursor = await loadCursor();
    if (!mounted) return;
    await _fetch();
    _subscribe(cursor);
  }

  Future<void> _fetch() async {
    try {
      final board = await widget.api.board();
      if (!mounted) return;
      setState(() {
        _board = board;
        _error = null;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = '$e');
    }
  }

  void _subscribe(int cursor) {
    unawaited(_sub?.cancel());
    _sub = widget.api.events(cursor: cursor).listen(
      (frame) async {
        if (frame.id != null) await saveCursor(frame.id!);
        await _fetch();
      },
      onError: (Object e) {
        if (!mounted) return;
        setState(() => _error = '$e');
      },
    );
  }

  Future<void> _refresh() async {
    await _fetch();
    _subscribe(await loadCursor());
  }

  @override
  Widget build(BuildContext context) {
    final board = _board;
    if (board == null && _error == null) {
      return const Center(child: CircularProgressIndicator());
    }
    if (board == null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            HxText('Board unreachable: ${_error ?? 'unknown'}',
                role: HxTextRole.caption),
            const SizedBox(height: 8),
            FilledButton(onPressed: _boot, child: const Text('Retry')),
          ],
        ),
      );
    }
    if (board.workers.isEmpty && board.prs.isEmpty) {
      return RefreshIndicator(
        onRefresh: _refresh,
        child: ListView(
          children: const [
            HxEmpty(
              icon: Icons.inbox_outlined,
              title: 'Nothing on the board',
              hint: 'Run a goal on the desktop; workers appear here.',
            ),
          ],
        ),
      );
    }
    return RefreshIndicator(
      onRefresh: _refresh,
      child: ListView(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.all(8),
        children: [
          for (final col in _columns)
            _Column(
              title: _label(col),
              workers: [
                for (final w in board.workers)
                  if (w.column == col) w
              ],
              prs: [
                for (final p in board.prs)
                  if (p.column == col) p
              ],
            ),
        ],
      ),
    );
  }
}

class _Column extends StatelessWidget {
  final String title;
  final List<HxWorkerCard> workers;
  final List<HxPrCard> prs;
  const _Column(
      {required this.title, required this.workers, required this.prs});

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 260,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.all(8),
            child: HxText('$title (${workers.length + prs.length})',
                role: HxTextRole.title),
          ),
          Expanded(
            child: ListView(
              children: [
                for (final w in workers) _WorkerTile(worker: w),
                for (final p in prs) _PrTile(pr: p),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _WorkerTile extends StatelessWidget {
  final HxWorkerCard worker;
  const _WorkerTile({required this.worker});

  @override
  Widget build(BuildContext context) {
    final status =
        worker.completed ? 'finished' : (worker.blocked ?? 'running');
    return HxCard(
      onTap: () => showModalBottomSheet<void>(
        context: context,
        builder: (_) => _WorkerSheet(worker: worker),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          HxText(worker.workerId, role: HxTextRole.mono),
          const SizedBox(height: 4),
          HxText(status, role: HxTextRole.caption),
        ],
      ),
    );
  }
}

class _PrTile extends StatelessWidget {
  final HxPrCard pr;
  const _PrTile({required this.pr});

  @override
  Widget build(BuildContext context) {
    final hx = context.hx;
    return HxCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          HxText(pr.repo.isEmpty ? '#${pr.number}' : '${pr.repo}#${pr.number}',
              role: HxTextRole.mono),
          const SizedBox(height: 4),
          HxText(pr.title, role: HxTextRole.body, maxLines: 2),
          const SizedBox(height: 6),
          Wrap(
            spacing: 6,
            children: [
              HxBadge(
                label: pr.checksGreen ? 'checks' : 'failing',
                color: pr.checksGreen ? hx.success : hx.error,
              ),
              if (pr.unresolved > 0)
                HxBadge(label: '${pr.unresolved} threads', color: hx.warning),
            ],
          ),
        ],
      ),
    );
  }
}

class _WorkerSheet extends StatelessWidget {
  final HxWorkerCard worker;
  const _WorkerSheet({required this.worker});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          HxText(worker.workerId, role: HxTextRole.title),
          const SizedBox(height: 8),
          HxText('Status: ${worker.column}', role: HxTextRole.body),
          HxText('Liveness: ${worker.alive ? 'alive' : 'stopped'}',
              role: HxTextRole.body),
          HxText('Blocker: ${worker.blocked ?? 'none'}', role: HxTextRole.body),
          HxText('Completed: ${worker.completed ? 'yes' : 'no'}',
              role: HxTextRole.body),
        ],
      ),
    );
  }
}
