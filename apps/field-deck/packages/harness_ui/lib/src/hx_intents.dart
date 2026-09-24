// Offline intent queue: phone actions that replay on reconnect.
// Pure Dart (dart: only). Each intent carries a client-generated
// idempotency id so replays never double-apply; conflicts surface as
// explicit choices (rebase, drop, escalate), never silent overwrites.
import 'dart:math';

/// Phone actions the daemon will accept once the intent endpoint lands.
/// `comment` and `approve` ride the lease-scoped forge path; `retry`
/// re-queues a worker unit for its next turn.
enum HxIntentKind { approvePr, retryWorker, comment }

/// Outcome of replaying one intent against a moved world.
/// `unsupported` means the daemon cannot execute the kind yet — the
/// reason names the follow-up, and the intent stays queued.
enum HxConflict { none, staleTarget, alreadyApplied, unsupported }

class HxIntent {
  final String idempotencyId;
  final HxIntentKind kind;
  final String repo;
  final int number;
  final String body;
  final DateTime createdAt;
  final HxConflict conflict;

  const HxIntent({
    required this.idempotencyId,
    required this.kind,
    required this.repo,
    required this.number,
    this.body = '',
    required this.createdAt,
    this.conflict = HxConflict.none,
  });

  /// Client-generated id: wall-clock plus 64 random bits, hex.
  static String newId([DateTime? now]) {
    final rng = Random.secure();
    final hi = rng.nextInt(1 << 32).toRadixString(16).padLeft(8, '0');
    final lo = rng.nextInt(1 << 32).toRadixString(16).padLeft(8, '0');
    final ts = (now ?? DateTime.now()).millisecondsSinceEpoch.toRadixString(16);
    return '$ts-$hi$lo';
  }

  Map<String, dynamic> toJson() => {
    'idempotency_id': idempotencyId,
    'kind': kind.name,
    'repo': repo,
    'number': number,
    'body': body,
    'created_at': createdAt.toIso8601String(),
    'conflict': conflict.name,
  };

  factory HxIntent.fromJson(Map<String, dynamic> json) => HxIntent(
    idempotencyId: json['idempotency_id'] as String? ?? '',
    kind: HxIntentKind.values.firstWhere(
      (k) => k.name == json['kind'],
      orElse: () => HxIntentKind.comment,
    ),
    repo: json['repo'] as String? ?? '',
    number: (json['number'] as num? ?? 0).toInt(),
    body: json['body'] as String? ?? '',
    createdAt:
        DateTime.tryParse(json['created_at'] as String? ?? '') ??
        DateTime.fromMillisecondsSinceEpoch(0),
    conflict: HxConflict.values.firstWhere(
      (c) => c.name == json['conflict'],
      orElse: () => HxConflict.none,
    ),
  );
}

/// In-memory queue; the app persists `toJson()` into shared_preferences
/// and replays FIFO on reconnect, newest conflict state per intent.
class HxIntentQueue {
  final List<HxIntent> _items = [];

  List<HxIntent> get pending => List.unmodifiable(_items);
  bool get isEmpty => _items.isEmpty;
  int get length => _items.length;

  void enqueue(HxIntent intent) {
    if (_items.any((i) => i.idempotencyId == intent.idempotencyId)) return;
    _items.add(intent);
  }

  bool remove(String idempotencyId) {
    final before = _items.length;
    _items.removeWhere((i) => i.idempotencyId == idempotencyId);
    return _items.length < before;
  }

  void markConflict(String idempotencyId, HxConflict conflict) {
    final idx = _items.indexWhere((i) => i.idempotencyId == idempotencyId);
    if (idx < 0) return;
    _items[idx] = HxIntent(
      idempotencyId: _items[idx].idempotencyId,
      kind: _items[idx].kind,
      repo: _items[idx].repo,
      number: _items[idx].number,
      body: _items[idx].body,
      createdAt: _items[idx].createdAt,
      conflict: conflict,
    );
  }

  List<Map<String, dynamic>> toJson() => [for (final i in _items) i.toJson()];

  static HxIntentQueue fromJson(List<dynamic> raw) {
    final queue = HxIntentQueue();
    for (final item in raw) {
      if (item is Map<String, dynamic>) queue.enqueue(HxIntent.fromJson(item));
    }
    return queue;
  }
}
