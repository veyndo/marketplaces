/** Source-order review vocabulary in all supported interface languages. */
export const orderWords = {
  grossMinor: [
    "Line gross amount · minor units",
    "Zeilenbrutto · kleinste Einheit",
    "Montant brut · unités mineures",
    "Bruto · unidades menores",
  ],
  taxMinor: [
    "Tax amount · minor units",
    "Steuerbetrag · kleinste Einheit",
    "Taxe · unités mineures",
    "Impuesto · unidades menores",
  ],
  taxRateBasisPoints: [
    "Tax rate · basis points (1900 = 19%)",
    "Steuersatz · Basispunkte (1900 = 19 %)",
    "Taux · points de base (1900 = 19 %)",
    "Tasa · puntos básicos (1900 = 19 %)",
  ],
  email: ["Buyer email", "Käufer-E-Mail", "E-mail acheteur", "Email comprador"],
  billingAddress: [
    "Billing address",
    "Rechnungsadresse",
    "Adresse de facturation",
    "Dirección de facturación",
  ],
  shippingAddress: [
    "Shipping address",
    "Lieferadresse",
    "Adresse de livraison",
    "Dirección de envío",
  ],
  street: [
    "Street & number",
    "Straße & Hausnummer",
    "Rue et numéro",
    "Calle y número",
  ],
  postalCode: ["Postal code", "Postleitzahl", "Code postal", "Código postal"],
  city: ["City", "Ort", "Ville", "Ciudad"],
  country: [
    "Country ISO code",
    "Länder-ISO-Code",
    "Code ISO pays",
    "Código ISO país",
  ],
  quantity: ["Quantity", "Menge", "Quantité", "Cantidad"],
  shipping: [
    "Shipping amounts",
    "Versandbeträge",
    "Montants livraison",
    "Importes de envío",
  ],
  totalMinor: [
    "Order total · minor units",
    "Bestellsumme · kleinste Einheit",
    "Total · unités mineures",
    "Total · unidades menores",
  ],
  paid: [
    "Payment confirmed by seller",
    "Zahlung beim Anbieter bestätigt",
    "Paiement confirmé",
    "Pago confirmado",
  ],
  unmapped: [
    "Seller SKUs without a unique product match",
    "Händler-SKUs ohne eindeutige Produktzuordnung",
    "SKU sans produit unique",
    "SKU sin producto único",
  ],
} as const;
