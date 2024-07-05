import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:security_center/app_permissions/interface_x.dart';
import 'package:security_center/app_permissions/rules_providers.dart';
import 'package:security_center/l10n.dart';
import 'package:security_center/navigator.dart';
import 'package:security_center/widgets/scrollable_page.dart';
import 'package:security_center/widgets/tile_list.dart';
import 'package:yaru/yaru.dart';

class InterfacesPage extends ConsumerWidget {
  const InterfacesPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final model = ref.watch(interfaceSnapCountsProvider);
    return model.when(
      data: (interfaceSnapCounts) =>
          _Body(interfaceSnapCounts: interfaceSnapCounts),
      error: (error, _) => ErrorWidget(error),
      loading: () => const Center(child: YaruCircularProgressIndicator()),
    );
  }
}

class _Body extends StatelessWidget {
  const _Body({required this.interfaceSnapCounts});

  final Map<String, int> interfaceSnapCounts;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final tiles = interfaceSnapCounts.entries
        .map(
          (interfaceSnapCount) => ListTile(
            contentPadding: const EdgeInsets.all(16),
            leading: Icon(interfaceSnapCount.key.snapdInterfaceIcon, size: 48),
            title:
                Text(interfaceSnapCount.key.localizeSnapdInterfaceTitle(l10n)),
            subtitle: Text(l10n.interfaceSnapCount(interfaceSnapCount.value)),
            trailing: const Icon(YaruIcons.pan_end),
            onTap: () => Navigator.of(context)
                .pushSnapPermissions(interface: interfaceSnapCount.key),
          ),
        )
        .toList();
    return ScrollablePage(
      children: [
        YaruBorderContainer(
          child: YaruSwitchListTile(
            title: Row(
              children: [
                Text(l10n.snapPermissionsEnableTitle),
                const SizedBox(width: 10),
                YaruInfoBadge(
                  title: Text(l10n.snapPermissionsExperimentalLabel),
                  yaruInfoType: YaruInfoType.information,
                ),
              ],
            ),
            subtitle: Text(l10n.snapPermissionsEnableWarning),
            value: true,
            onChanged: (value) {},
          ),
        ),
        const SizedBox(height: 16),
        TileList(children: tiles),
        const SizedBox(height: 16),
        Text(l10n.snapPermissionsOtherDescription),
      ],
    );
  }
}