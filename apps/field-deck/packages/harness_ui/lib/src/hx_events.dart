// SSE event-stream parser with cursor tracking for the live feed.
// Pure Dart (dart: only). `event:`, `data:`, and `id:` lines build one
// frame per blank line; `:` comments are ignored; multi-line data joins
// with newlines. The board screen wires this to GET /api/v1/events next
// (stream transport rides with the SDK verification pass).
import 'dart:convert';

/// One server-sent frame: daemon kind, JSON summary, sequence cursor.
class HxServerEvent {
  final String kind;
  final String data;
  final int? id;
  const HxServerEvent({required this.kind, required this.data, this.id});

  /// Decoded JSON summary, or empty when the frame is not JSON.
  Map<String, dynamic> get json {
    try {
      final decoded = jsonDecode(data);
      if (decoded is Map<String, dynamic>) return decoded;
    } catch (_) {}
    return const {};
  }
}

/// Incremental parser: feed raw lines, drain complete frames.
class HxEventParser {
  String? _event;
  final List<String> _data = [];
  int? _id;

  /// Parse one stream line; returns a frame on a dispatching blank line.
  /// Tolerates CRLF wire endings (the trailing return never leaks).
  HxServerEvent? addLine(String line) {
    if (line.endsWith('\r')) line = line.substring(0, line.length - 1);
    if (line.isEmpty) return dispatch();
    if (line.startsWith(':')) return null;
    final colon = line.indexOf(':');
    if (colon < 0) return null;
    final field = line.substring(0, colon);
    var value = line.substring(colon + 1);
    if (value.startsWith(' ')) value = value.substring(1);
    switch (field) {
      case 'event':
        _event = value;
      case 'data':
        _data.add(value);
      case 'id':
        _id = int.tryParse(value) ?? _id;
    }
    return null;
  }

  /// Dispatch the buffered frame (also called by trailing blank lines).
  HxServerEvent? dispatch() {
    if (_event == null && _data.isEmpty) return null;
    final frame = HxServerEvent(
      kind: _event ?? 'message',
      data: _data.join('\n'),
      id: _id,
    );
    _event = null;
    _data.clear();
    _id = null;
    return frame;
  }

  /// Newest cursor seen; persist it and resume with `?cursor=N`.
  int cursor = 0;

  /// Feed one frame's cursor forward. Returns the frame for chaining.
  HxServerEvent? track(HxServerEvent? frame) {
    if (frame?.id != null) cursor = frame!.id!;
    return frame;
  }
}
