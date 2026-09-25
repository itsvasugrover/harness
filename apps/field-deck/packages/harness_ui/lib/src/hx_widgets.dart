// Harness widgets: one file per widget family would balloon the kit,
// so this module holds the four primitives every screen shares.
// All text roles, badges, and empty states come from here — screens
// never hand-roll text styles or pill decorations.
import 'package:flutter/material.dart';
import 'hx_theme.dart';

/// Card with kit-standard margin and hairline border (see HxTheme).
class HxCard extends StatelessWidget {
  final Widget child;
  final VoidCallback? onTap;
  const HxCard({super.key, required this.child, this.onTap});

  @override
  Widget build(BuildContext context) {
    final card = Card(
      child: Padding(padding: const EdgeInsets.all(12), child: child),
    );
    if (onTap == null) return card;
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(12),
      child: card,
    );
  }
}

/// Role-based text: screens pick a role, never a raw TextStyle.
enum HxTextRole { title, body, caption, mono }

class HxText extends StatelessWidget {
  final String data;
  final HxTextRole role;
  final int? maxLines;
  const HxText(
    this.data, {
    super.key,
    this.role = HxTextRole.body,
    this.maxLines,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context).textTheme;
    final muted = context.hx.muted;
    final style = switch (role) {
      HxTextRole.title => theme.titleSmall,
      HxTextRole.body => theme.bodyMedium,
      HxTextRole.caption => theme.bodySmall?.copyWith(color: muted),
      HxTextRole.mono => theme.bodySmall?.copyWith(
          fontFamily: 'monospace',
          color: muted,
        ),
    };
    return Text(
      data,
      style: style,
      maxLines: maxLines,
      overflow: maxLines == null ? null : TextOverflow.ellipsis,
    );
  }
}

/// Pill badge with an explicit tone — status only, never decoration.
class HxBadge extends StatelessWidget {
  final String label;
  final Color color;
  const HxBadge({super.key, required this.label, required this.color});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.16),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        label,
        style: Theme.of(context).textTheme.labelSmall?.copyWith(color: color),
      ),
    );
  }

  /// Verdict badge: pass/warn/block tones from the palette.
  static HxBadge verdict(BuildContext context, String verdict) {
    final hx = context.hx;
    final lower = verdict.toLowerCase();
    final color = lower == 'pass'
        ? hx.success
        : lower == 'warn'
            ? hx.warning
            : lower == 'block'
                ? hx.error
                : hx.muted;
    return HxBadge(label: verdict.isEmpty ? 'pending' : verdict, color: color);
  }
}

/// Honest empty state: icon, title, and what unlocks content.
class HxEmpty extends StatelessWidget {
  final IconData icon;
  final String title;
  final String hint;
  const HxEmpty({
    super.key,
    required this.icon,
    required this.title,
    required this.hint,
  });

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 40, color: context.hx.muted),
            const SizedBox(height: 12),
            HxText(title, role: HxTextRole.title),
            const SizedBox(height: 4),
            HxText(hint, role: HxTextRole.caption),
          ],
        ),
      ),
    );
  }
}
