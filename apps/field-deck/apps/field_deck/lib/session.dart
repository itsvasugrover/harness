// Field Deck session: host URL, bearer token, and intent persistence.
// Host URL lives in shared_preferences (non-secret); the bearer lives
// in flutter_secure_storage and never in logs, prefs, or backups.
import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:harness_ui/harness_ui.dart';
import 'package:shared_preferences/shared_preferences.dart';

const _hostKey = 'hx_host';
const _bearerKey = 'hx_bearer';
const _queueKey = 'hx_intent_queue';
const _defaultHost = 'http://127.0.0.1:4317';

class HxConnection {
  final String baseUrl;
  final String bearer;
  const HxConnection({required this.baseUrl, required this.bearer});

  HxApi get api => HxApi(baseUrl: baseUrl, bearer: bearer);
}

Future<HxConnection> loadConnection() async {
  final prefs = await SharedPreferences.getInstance();
  const storage = FlutterSecureStorage();
  final host = prefs.getString(_hostKey) ?? _defaultHost;
  final bearer = await storage.read(key: _bearerKey) ?? '';
  return HxConnection(baseUrl: host, bearer: bearer);
}

Future<void> saveConnection(String host, String bearer) async {
  final prefs = await SharedPreferences.getInstance();
  const storage = FlutterSecureStorage();
  await prefs.setString(_hostKey, host);
  if (bearer.isEmpty) {
    await storage.delete(key: _bearerKey);
  } else {
    await storage.write(key: _bearerKey, value: bearer);
  }
}

Future<HxIntentQueue> loadQueue() async {
  final prefs = await SharedPreferences.getInstance();
  final raw = prefs.getString(_queueKey);
  if (raw == null || raw.isEmpty) return HxIntentQueue();
  try {
    final decoded = jsonDecode(raw);
    if (decoded is! List) return HxIntentQueue();
    return HxIntentQueue.fromJson(decoded);
  } catch (_) {
    // Corrupt queue drops rather than wedging the app.
    return HxIntentQueue();
  }
}

Future<void> saveQueue(HxIntentQueue queue) async {
  final prefs = await SharedPreferences.getInstance();
  await prefs.setString(_queueKey, jsonEncode(queue.toJson()));
}
