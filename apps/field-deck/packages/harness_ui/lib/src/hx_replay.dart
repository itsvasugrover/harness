// Intent replay: drain the offline queue through POST /api/v1/intents.
// Pure Dart (dart: only). Applied intents leave the queue; conflicts
// attach explicit choices for the rebase/drop/escalate decision; a
// transport failure stops the drain so order — and the queue — survive.
import 'hx_api.dart';
import 'hx_intents.dart';

class HxReplayReport {
  final int applied;
  final int conflicted;
  final List<String> lines;
  const HxReplayReport({
    required this.applied,
    required this.conflicted,
    required this.lines,
  });

  String get summary {
    if (lines.isEmpty) return 'queue empty — nothing to sync';
    return 'synced: $applied applied, $conflicted conflicted';
  }
}

String _label(HxIntent intent) {
  final target = intent.repo.isEmpty
      ? '#${intent.number}'
      : '${intent.repo}#${intent.number}';
  return '${intent.kind.name} $target';
}

Future<HxReplayReport> replayQueue(HxApi api, HxIntentQueue queue) async {
  var applied = 0;
  var conflicted = 0;
  final lines = <String>[];
  for (final intent in List.of(queue.pending)) {
    final kind = switch (intent.kind) {
      HxIntentKind.approvePr => 'approve_pr',
      HxIntentKind.retryWorker => 'retry_worker',
      HxIntentKind.comment => 'comment',
    };
    HxIntentResult res;
    try {
      res = await api.postIntent({
        'idempotency_id': intent.idempotencyId,
        'kind': kind,
        'repo': intent.repo,
        'number': intent.number,
        'body': intent.body,
      });
    } catch (e) {
      lines.add(
        'offline — ${_label(intent)} kept (${e is HxApiException ? e.message : e})',
      );
      break;
    }
    if (res.applied) {
      queue.remove(intent.idempotencyId);
      applied++;
      lines.add('applied ${_label(intent)}: ${res.summary}');
    } else if (res.status == 409) {
      queue.markConflict(intent.idempotencyId, HxConflict.staleTarget);
      conflicted++;
      lines.add('conflict ${_label(intent)}: ${res.summary}');
    } else {
      queue.markConflict(intent.idempotencyId, HxConflict.unsupported);
      conflicted++;
      lines.add('unsupported ${_label(intent)}: ${res.summary}');
    }
  }
  return HxReplayReport(applied: applied, conflicted: conflicted, lines: lines);
}
