// Audit tab: Sentinel verdicts plus the append-only log viewer.
// Read-only by rule; the phone judges nothing, it surfaces.
import 'package:flutter/material.dart';
import 'package:harness_ui/harness_ui.dart';

class HxAuditScreen extends StatelessWidget {
  final HxApi api;
  const HxAuditScreen({super.key, required this.api});

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<List<HxAuditEvent>>(
      future: api.audit(),
      builder: (context, snap) {
        if (snap.connectionState == ConnectionState.waiting) {
          return const Center(child: CircularProgressIndicator());
        }
        if (snap.hasError || !snap.hasData || snap.data!.isEmpty) {
          return HxEmpty(
            icon: Icons.fact_check_outlined,
            title: 'No audit events',
            hint: '${snap.error ?? 'Every consequential action lands here.'}',
          );
        }
        return ListView(
          children: [
            for (final e in snap.data!)
              HxCard(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        HxBadge.verdict(context, e.verdict),
                        const SizedBox(width: 6),
                        HxText('#${e.seq} ${e.kind}', role: HxTextRole.mono),
                        const Spacer(),
                        HxText(
                          e.time.length > 16 ? e.time.substring(0, 16) : e.time,
                          role: HxTextRole.caption,
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    HxText(e.summary, role: HxTextRole.body, maxLines: 3),
                    const SizedBox(height: 2),
                    HxText('${e.actor} · ${e.agent}', role: HxTextRole.caption),
                  ],
                ),
              ),
          ],
        );
      },
    );
  }
}
