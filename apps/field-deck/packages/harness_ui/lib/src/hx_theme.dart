// Harness theme: seed-color Material 3 light/dark schemes plus the
// semantic tokens screens read through `context.hx`. Hardcoded color
// values live only in this file.
import 'package:flutter/material.dart';

/// Brand seed: deep telemetry blue. One accent, ≤10% of any screen.
const Color hxSeed = Color(0xFF2F5FE3);

class HxTheme {
  static ThemeData light() {
    final scheme = ColorScheme.fromSeed(seedColor: hxSeed);
    return ThemeData(
      useMaterial3: true,
      colorScheme: scheme,
      scaffoldBackgroundColor: scheme.surface,
      appBarTheme: AppBarTheme(
        backgroundColor: scheme.surface,
        foregroundColor: scheme.onSurface,
      ),
      cardTheme: CardThemeData(
        margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: BorderSide(color: scheme.outlineVariant),
        ),
      ),
    );
  }

  static ThemeData dark() {
    final scheme = ColorScheme.fromSeed(
      seedColor: hxSeed,
      brightness: Brightness.dark,
    );
    return ThemeData(
      useMaterial3: true,
      colorScheme: scheme,
      scaffoldBackgroundColor: scheme.surface,
      appBarTheme: AppBarTheme(
        backgroundColor: scheme.surface,
        foregroundColor: scheme.onSurface,
      ),
      cardTheme: CardThemeData(
        margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: BorderSide(color: scheme.outlineVariant),
        ),
      ),
    );
  }
}

/// Semantic tokens: status colors that adapt to the active brightness.
class HxPalette {
  final bool dark;
  const HxPalette(this.dark);

  Color get success => dark ? const Color(0xFF4ADE80) : const Color(0xFF15803D);
  Color get warning => dark ? const Color(0xFFFBBF24) : const Color(0xFFB45309);
  Color get error => dark ? const Color(0xFFF87171) : const Color(0xFFB91C1C);
  Color get muted => dark ? const Color(0xFF9CA3AF) : const Color(0xFF6B7280);
}

extension HxContext on BuildContext {
  HxPalette get hx => HxPalette(Theme.of(this).brightness == Brightness.dark);
}
