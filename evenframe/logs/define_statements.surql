
DEFINE TABLE OVERWRITE tax_rate SCHEMAFULL TYPE NORMAL CHANGEFEED 3d PERMISSIONS FOR select WHERE $auth.permissions.data.any('TaxRate') = true FOR update WHERE $auth.permissions.data.any('TaxRate') = true FOR create WHERE $auth.permissions.data.any('TaxRate') = true FOR delete WHERE $auth.permissions.data.any('TaxRate') = true;
DEFINE FIELD OVERWRITE name ON TABLE tax_rate TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE tax_agency ON TABLE tax_rate TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE zip ON TABLE tax_rate TYPE int DEFAULT 0 PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE city ON TABLE tax_rate TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE county ON TABLE tax_rate TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE state ON TABLE tax_rate TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE is_active ON TABLE tax_rate TYPE bool DEFAULT false PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE description ON TABLE tax_rate TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE tax_components ON TABLE tax_rate TYPE object DEFAULT {} PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD tax_components.* ON TABLE tax_rate TYPE float;

DEFINE TABLE OVERWRITE company SCHEMAFULL TYPE NORMAL CHANGEFEED 3d PERMISSIONS FOR select WHERE $auth.permissions.data.any('Company') = true FOR update WHERE $auth.permissions.data.any('Company') = true FOR create WHERE $auth.permissions.data.any('Company') = true FOR delete WHERE $auth.permissions.data.any('Company') = true;
DEFINE FIELD OVERWRITE legal_name ON TABLE company TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE phones ON TABLE company TYPE array<{ main: bool, phone_type: string, number: string, can_text: bool, can_call: bool }> DEFAULT [] PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE fax ON TABLE company TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE email ON TABLE company TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE website ON TABLE company TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE tax_id ON TABLE company TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE reference_number ON TABLE company TYPE int DEFAULT 0 PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE postal_code_lookup ON TABLE company TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE time_zone ON TABLE company TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE default_tax ON TABLE company TYPE record<tax_rate> DEFAULT NULL PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE default_tax_location ON TABLE company TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE default_area_code ON TABLE company TYPE int DEFAULT 0 PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE default_account_type ON TABLE company TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE lookup_formatting ON TABLE company TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE account_name_format ON TABLE company TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE merchant_service_provider ON TABLE company TYPE null | string DEFAULT NULL PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE date_display_style ON TABLE company TYPE string DEFAULT '' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE has_auto_commission ON TABLE company TYPE bool DEFAULT false PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE has_auto_daylight_savings ON TABLE company TYPE bool DEFAULT false PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE has_auto_fms_tracking ON TABLE company TYPE bool DEFAULT false PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE has_notifications ON TABLE company TYPE bool DEFAULT false PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE has_required_lead_source ON TABLE company TYPE bool DEFAULT false PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE has_required_email ON TABLE company TYPE bool DEFAULT false PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE has_sort_service_items_alphabetically ON TABLE company TYPE bool DEFAULT false PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE has_attach_order_to_appointment_emails ON TABLE company TYPE bool DEFAULT false PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;
DEFINE FIELD OVERWRITE schedule_interval ON TABLE company TYPE int DEFAULT 0 PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;

DEFINE TABLE OVERWRITE payment SCHEMAFULL TYPE NORMAL CHANGEFEED 3d PERMISSIONS FOR select WHERE $auth.permissions.data.any('Payment') = true FOR update WHERE $auth.permissions.data.any('Payment') = true FOR create WHERE $auth.permissions.data.any('Payment') = true FOR delete WHERE $auth.permissions.data.any('Payment') = true;
DEFINE FIELD OVERWRITE date ON TABLE payment TYPE datetime DEFAULT d'2024-01-01T00:00:00Z' PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;

