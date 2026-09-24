// Review inbox: PRs needing a human, derived from board facts.
// Approve queues an idempotent intent that replays on reconnect;
// the queue (not the network) is the source of truth offline.
import 'package:flutter/material.dart';
import 'package:harness_ui/harness_ui.dart';

class HxReviewScreen extends StatelessWidget {
  final HxApi api;
  final void Function(HxIntent intent) onQueue;
  const HxReviewScreen({super.key, required this.api, required this.onQueue});

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<HxBoard>(
      future: api.board(),
      builder: (context, snap) {
        if (snap.connectionState == ConnectionState.waiting) {
          return const Center(child: CircularProgressIndicator());
        }
        if (snap.hasError || !snap.hasData) {
          return HxEmpty(
            icon: Icons.cloud_off_outlined,
            title: 'Review inbox unreachable',
            hint: '${snap.error ?? 'unknown error'}',
          );
        }
        final needs = [
          for (final p in snap.data!.prs)
            if (!p.checksGreen || p.unresolved > 0) p,
        ];
        if (needs.isEmpty) {
          return const HxEmpty(
            icon: Icons.rate_review_outlined,
            title: 'Inbox zero',
            hint: 'No PR is failing checks or holding open threads.',
          );
        }
        return ListView(
          children: [
            for (final p in needs)
              HxCard(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    HxText(
                      p.repo.isEmpty ? '#${p.number}' : '${p.repo}#${p.number}',
                      role: HxTextRole.mono,
                    ),
                    const SizedBox(height: 4),
                    HxText(p.title, role: HxTextRole.body, maxLines: 2),
                    const SizedBox(height: 8),
                    Row(
                      children: [
                        HxBadge(
                          label: p.checksGreen ? 'checks' : 'failing',
                          color: p.checksGreen
                              ? context.hx.success
                              : context.hx.error,
                        ),
                        const SizedBox(width: 6),
                        if (p.unresolved > 0)
                          HxBadge(
                            label: '${p.unresolved} threads',
                            color: context.hx.warning,
                          ),
                        const Spacer(),
                        TextButton(
                          onPressed: () {
                            onQueue(
                              HxIntent(
                                idempotencyId: HxIntent.newId(),
                                kind: HxIntentKind.approvePr,
                                repo: p.repo,
                                number: p.number,
                                createdAt: DateTime.now(),
                              ),
                            );
                            ScaffoldMessenger.of(context).showSnackBar(
                              const SnackBar(
                                content: Text(
                                  'Approval queued — syncs on reconnect',
                                ),
                              ),
                            );
                          },
                          child: const Text('Approve'),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
          ],
        );
      },
    );
  }
}
