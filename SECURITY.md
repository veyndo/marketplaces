# Security

Report vulnerabilities privately to the repository owner through GitHub private
vulnerability reporting when available, or contact the Veyndo maintainer before
publishing credentials or reproduction data. Never open an issue containing seller
tokens, callback keys, customer addresses or order payloads from a real shop.

The operator gateway token grants access to configured tenants. Keep it server
side. Host callback credentials must enforce current permissions, consent and
revocation. Do not turn the synthetic test host into production storage. Review
uncertain external writes with the seller before retrying.

Security fixes are maintained on main and identified in component releases.
