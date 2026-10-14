/** Guided seller setup copy; credentials remain service-side and installation does not imply a live seller connection. */
import { useLocale } from "../../../../frontend/src/shared/i18n/i18n";
const words = {
  studio: [
    "Marketplace Studio",
    "Marketplace Studio",
    "Marketplace Studio",
    "Marketplace Studio",
  ],
  intro: [
    "Your channel is ready. Connect a marketplace, review your offers and synchronize orders here.",
    "Dein Kanal ist bereit. Verbinde einen Marktplatz, prüfe deine Angebote und synchronisiere hier Bestellungen.",
    "Votre canal est prêt. Connectez une marketplace, vérifiez vos offres et synchronisez les commandes ici.",
    "Tu canal está listo. Conecta un marketplace, revisa ofertas y sincroniza pedidos aquí.",
  ],
  loading: [
    "Loading Marketplace Studio…",
    "Marketplace Studio wird geladen…",
    "Chargement de Marketplace Studio…",
    "Cargando Marketplace Studio…",
  ],
  rights: [
    "Marketplace setup requires app read and management permissions. Your channel remains saved.",
    "Für die Marktplatz-Einrichtung brauchst du Lese- und Verwaltungsrechte für Apps. Dein Kanal bleibt gespeichert.",
    "La configuration nécessite les droits de lecture et gestion des apps. Votre canal reste enregistré.",
    "La configuración requiere permisos de lectura y gestión de apps. Tu canal sigue guardado.",
  ],
  connect: [
    "Connect your first marketplace",
    "Verbinde deinen ersten Marktplatz",
    "Connectez votre première marketplace",
    "Conecta tu primer marketplace",
  ],
  setup: [
    "Enable the Marketplace app once for this shop. Your new channel will be selected in the connection form.",
    "Aktiviere die Marketplace-App einmal für diesen Shop. Dein neuer Kanal ist im Verbindungsformular bereits ausgewählt.",
    "Activez l’app Marketplace une fois pour cette boutique. Votre nouveau canal sera présélectionné.",
    "Activa la app Marketplace una vez para esta tienda. Tu nuevo canal estará seleccionado.",
  ],
  step1: [
    "Review and enable the app",
    "App prüfen und aktivieren",
    "Vérifier et activer l’app",
    "Revisar y activar la app",
  ],
  step2: [
    "Choose Amazon, eBay or Kaufland and your seller account",
    "Amazon, eBay oder Kaufland und dein Verkäuferkonto auswählen",
    "Choisir Amazon, eBay ou Kaufland et votre compte vendeur",
    "Elegir Amazon, eBay o Kaufland y tu cuenta de vendedor",
  ],
  step3: [
    "Review product mappings before synchronization",
    "Produktzuordnung vor der Synchronisierung prüfen",
    "Vérifier les correspondances avant synchronisation",
    "Revisar asignaciones antes de sincronizar",
  ],
  install: [
    "Set up Marketplace Studio",
    "Marketplace Studio einrichten",
    "Configurer Marketplace Studio",
    "Configurar Marketplace Studio",
  ],
  activate: [
    "Activate Marketplace Studio",
    "Marketplace Studio aktivieren",
    "Activer Marketplace Studio",
    "Activar Marketplace Studio",
  ],
} as const;
export function useMarketplaceSetupText() {
  const { locale } = useLocale();
  const i =
    locale === "de-DE"
      ? 1
      : locale === "fr-FR"
        ? 2
        : locale === "es-ES"
          ? 3
          : 0;
  return (key: keyof typeof words) => words[key][i];
}
