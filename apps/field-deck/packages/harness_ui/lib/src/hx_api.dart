// Harness daemon API: typed models + thin HttpClient wrapper.
// Pure Dart (dart: only, zero third-party deps) so the kit rule holds
// and this file analyzes without the Flutter SDK. Columns arrive
// server-derived; the phone never computes placement.
import 'dart:convert';
import 'dart:io';

/// Phone-side contract gate: refuse hosts that report a newer contract.
const int hxContract = 2;

/// Daemon identity probe (the single unauthenticated route).
class HxIdentity {
  final String hostId;
  final int contract;
  const HxIdentity({required this.hostId, required this.contract});

  factory HxIdentity.fromJson(Map<String, dynamic> json) => HxIdentity(
    hostId: json['host_id'] as String? ?? '',
    contract: (json['contract'] as num? ?? 0).toInt(),
  );
}

/// One derived board: worker cards plus observer PR cards.
class HxBoard {
  final List<HxWorkerCard> workers;
  final List<HxPrCard> prs;
  const HxBoard({required this.workers, required this.prs});

  factory HxBoard.fromJson(Map<String, dynamic> json) => HxBoard(
    workers: _list(json['workers'], HxWorkerCard.fromJson),
    prs: _list(json['prs'], HxPrCard.fromJson),
  );

  static List<T> _list<T>(dynamic raw, T Function(Map<String, dynamic>) parse) {
    if (raw is! List) return const [];
    return [
      for (final item in raw)
        if (item is Map<String, dynamic>) parse(item),
    ];
  }
}

class HxWorkerCard {
  final String workerId;
  final String column;
  final bool alive;
  final String? blocked;
  final bool completed;
  const HxWorkerCard({
    required this.workerId,
    required this.column,
    required this.alive,
    required this.blocked,
    required this.completed,
  });

  factory HxWorkerCard.fromJson(Map<String, dynamic> json) => HxWorkerCard(
    workerId: json['worker_id'] as String? ?? '',
    column: json['column'] as String? ?? 'needs_you',
    alive: json['alive'] as bool? ?? false,
    blocked: json['blocked'] as String?,
    completed: json['completed'] as bool? ?? false,
  );
}

class HxPrCard {
  final String repo;
  final int number;
  final String title;
  final String state;
  final String column;
  final bool checksGreen;
  final int unresolved;
  final bool mergeable;
  const HxPrCard({
    required this.repo,
    required this.number,
    required this.title,
    required this.state,
    required this.column,
    required this.checksGreen,
    required this.unresolved,
    required this.mergeable,
  });

  factory HxPrCard.fromJson(Map<String, dynamic> json) => HxPrCard(
    repo: json['repo'] as String? ?? '',
    number: (json['number'] as num? ?? 0).toInt(),
    title: json['title'] as String? ?? '',
    state: json['state'] as String? ?? '',
    column: json['column'] as String? ?? 'needs_you',
    checksGreen: json['checks_green'] as bool? ?? false,
    unresolved: (json['unresolved'] as num? ?? 0).toInt(),
    mergeable: json['mergeable'] as bool? ?? false,
  );
}

class HxAuditEvent {
  final int seq;
  final String id;
  final String time;
  final String actor;
  final String agent;
  final String? skill;
  final String repo;
  final String kind;
  final String summary;
  final String verdict;
  const HxAuditEvent({
    required this.seq,
    required this.id,
    required this.time,
    required this.actor,
    required this.agent,
    required this.skill,
    required this.repo,
    required this.kind,
    required this.summary,
    required this.verdict,
  });

  factory HxAuditEvent.fromJson(Map<String, dynamic> json) {
    final attr = json['attribution'];
    final map = attr is Map<String, dynamic> ? attr : <String, dynamic>{};
    return HxAuditEvent(
      seq: (json['seq'] as num? ?? 0).toInt(),
      id: json['id'] as String? ?? '',
      time: json['time'] as String? ?? '',
      actor: map['actor'] as String? ?? '',
      agent: map['agent'] as String? ?? '',
      skill: map['skill'] as String?,
      repo: json['repo'] as String? ?? '',
      kind: json['kind'] as String? ?? '',
      summary: json['summary'] as String? ?? '',
      verdict: json['verdict'] as String? ?? 'pending',
    );
  }
}

/// Transport failure with the daemon's answer attached when present.
class HxApiException implements Exception {
  final int status;
  final String message;
  const HxApiException(this.status, this.message);
  @override
  String toString() => 'HxApiException($status): $message';
}

/// Thin client over the versioned daemon API. Bearer goes on every
/// route except the identity probe; tokens live in secure storage and
/// never touch logs (see field_deck session handling).
class HxApi {
  final String baseUrl;
  final String bearer;
  final HttpClient _http = HttpClient();

  HxApi({required this.baseUrl, this.bearer = ''});

  void close() => _http.close(force: true);

  Future<HxIdentity> identity() async {
    final json = await _get('/api/v1/identity', authed: false);
    return HxIdentity.fromJson(json as Map<String, dynamic>);
  }

  Future<HxBoard> board() async {
    final json = await _get('/api/v1/board');
    return HxBoard.fromJson(json as Map<String, dynamic>);
  }

  Future<List<HxAuditEvent>> audit({int limit = 50}) async {
    final json = await _get('/api/v1/audit?limit=$limit');
    if (json is! List) return const [];
    return [
      for (final item in json)
        if (item is Map<String, dynamic>) HxAuditEvent.fromJson(item),
    ];
  }

  Future<dynamic> _get(String path, {bool authed = true}) async {
    final uri = Uri.parse('$baseUrl$path');
    final req = await _http.getUrl(uri).timeout(const Duration(seconds: 15));
    if (authed && bearer.isNotEmpty) {
      req.headers.set(HttpHeaders.authorizationHeader, 'Bearer $bearer');
    }
    final res = await req.close().timeout(const Duration(seconds: 15));
    final body = await res.transform(utf8.decoder).join();
    if (res.statusCode == HttpStatus.unauthorized) {
      throw const HxApiException(401, 'bearer rejected — re-pair in Settings');
    }
    if (res.statusCode != HttpStatus.ok) {
      throw HxApiException(res.statusCode, '$path failed (${res.statusCode})');
    }
    return jsonDecode(body);
  }
}
