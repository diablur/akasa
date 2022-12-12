mod test_01_connect_575314;
mod test_01_connect_allow_anonymous;
mod test_01_connect_disconnect_v5;
mod test_01_connect_max_connections;
mod test_01_connect_max_keepalive;
mod test_01_connect_take_over;
mod test_01_connect_uname_no_password_denied;
mod test_01_connect_uname_or_anon;
mod test_01_connect_uname_password_denied;
mod test_01_connect_uname_password_denied_no_will;
mod test_01_connect_uname_password_success_no_tls;
mod test_01_connect_windows_line_endings;
mod test_01_connect_zero_length_id;
mod test_02_shared_qos0_v5;
mod test_02_subhier_crash;
mod test_02_subpub_qos0_long_topic;
mod test_02_subpub_qos0_oversize_payload;
mod test_02_subpub_qos0_queued_bytes;
mod test_02_subpub_qos0_retain_as_publish;
mod test_02_subpub_qos0_send_retain;
mod test_02_subpub_qos0_subscription_id;
mod test_02_subpub_qos0_topic_alias;
mod test_02_subpub_qos0_topic_alias_unknown;
mod test_02_subpub_qos1;
mod test_02_subpub_qos1_message_expiry;
mod test_02_subpub_qos1_message_expiry_retain;
mod test_02_subpub_qos1_message_expiry_will;
mod test_02_subpub_qos1_nolocal;
mod test_02_subpub_qos1_oversize_payload;
mod test_02_subpub_qos2;
mod test_02_subpub_qos2_1322;
mod test_02_subpub_qos2_max_inflight_bytes;
mod test_02_subpub_qos2_pubrec_error;
mod test_02_subpub_qos2_receive_maximum_1;
mod test_02_subpub_qos2_receive_maximum_2;
mod test_02_subpub_qos2_receive_maximum_helper;
mod test_02_subpub_recover_subscriptions;
mod test_02_subscribe_dollar_v5;
mod test_02_subscribe_invalid_utf8;
mod test_02_subscribe_long_topic;
mod test_02_subscribe_persistence_flipflop;
mod test_03_pattern_matching;
mod test_03_publish_b2c_disconnect_qos1;
mod test_03_publish_b2c_disconnect_qos2;
mod test_03_publish_b2c_qos1_len;
mod test_03_publish_b2c_qos2_len;
mod test_03_publish_c2b_disconnect_qos2;
mod test_03_publish_c2b_qos2_len;
mod test_03_publish_dollar;
mod test_03_publish_dollar_v5;
mod test_03_publish_invalid_utf8;
mod test_03_publish_long_topic;
mod test_03_publish_qos1;
mod test_03_publish_qos1_max_inflight;
mod test_03_publish_qos1_max_inflight_expire;
mod test_03_publish_qos1_no_subscribers_v5;
mod test_03_publish_qos1_queued_bytes;
mod test_03_publish_qos1_retain_disabled;
mod test_03_publish_qos2;
mod test_03_publish_qos2_max_inflight;
mod test_04_retain_check_source;
mod test_04_retain_check_source_persist;
mod test_04_retain_check_source_persist_diff_port;
mod test_04_retain_qos0;
mod test_04_retain_qos0_clear;
mod test_04_retain_qos0_fresh;
mod test_04_retain_qos0_repeated;
mod test_04_retain_qos1_qos0;
mod test_04_retain_upgrade_outgoing_qos;
mod test_05_clean_session_qos1;
mod test_05_session_expiry_v5;
mod test_06_bridge_b2br_disconnect_qos1;
mod test_06_bridge_b2br_disconnect_qos2;
mod test_06_bridge_b2br_late_connection;
mod test_06_bridge_b2br_late_connection_retain;
mod test_06_bridge_b2br_remapping;
mod test_06_bridge_br2b_disconnect_qos1;
mod test_06_bridge_br2b_disconnect_qos2;
mod test_06_bridge_br2b_remapping;
mod test_06_bridge_clean_session_core;
mod test_06_bridge_clean_session_csF_lcsF;
mod test_06_bridge_clean_session_csF_lcsN;
mod test_06_bridge_clean_session_csF_lcsT;
mod test_06_bridge_clean_session_csT_lcsF;
mod test_06_bridge_clean_session_csT_lcsN;
mod test_06_bridge_clean_session_csT_lcsT;
mod test_06_bridge_fail_persist_resend_qos1;
mod test_06_bridge_fail_persist_resend_qos2;
mod test_06_bridge_no_local;
mod test_06_bridge_outgoing_retain;
mod test_06_bridge_per_listener_settings;
mod test_06_bridge_reconnect_local_out;
mod test_07_will_delay;
mod test_07_will_delay_invalid_573191;
mod test_07_will_delay_reconnect;
mod test_07_will_delay_recover;
mod test_07_will_delay_session_expiry;
mod test_07_will_delay_session_expiry2;
mod test_07_will_disconnect_with_will;
mod test_07_will_invalid_utf8;
mod test_07_will_no_flag;
mod test_07_will_null;
mod test_07_will_null_topic;
mod test_07_will_oversize_payload;
mod test_07_will_per_listener;
mod test_07_will_properties;
mod test_07_will_qos0;
mod test_07_will_reconnect_1273;
mod test_07_will_takeover;
mod test_08_ssl_bridge;
mod test_08_ssl_bridge_helper;
mod test_08_ssl_connect_cert_auth;
mod test_08_ssl_connect_cert_auth_crl;
mod test_08_ssl_connect_cert_auth_expired;
mod test_08_ssl_connect_cert_auth_revoked;
mod test_08_ssl_connect_cert_auth_without;
mod test_08_ssl_connect_identity;
mod test_08_ssl_connect_no_auth;
mod test_08_ssl_connect_no_auth_wrong_ca;
mod test_08_ssl_connect_no_identity;
mod test_08_ssl_hup_disconnect;
mod test_08_tls_psk_bridge;
mod test_08_tls_psk_pub;
mod test_09_acl_access_variants;
mod test_09_acl_change;
mod test_09_acl_empty_file;
mod test_09_auth_bad_method;
mod test_09_extended_auth_change_username;
mod test_09_extended_auth_multistep;
mod test_09_extended_auth_multistep_reauth;
mod test_09_extended_auth_reauth;
mod test_09_extended_auth_single;
mod test_09_extended_auth_single2;
mod test_09_plugin_acl_change;
mod test_09_plugin_auth_acl_pub;
mod test_09_plugin_auth_acl_sub;
mod test_09_plugin_auth_acl_sub_denied;
mod test_09_plugin_auth_context_params;
mod test_09_plugin_auth_defer_unpwd_fail;
mod test_09_plugin_auth_defer_unpwd_success;
mod test_09_plugin_auth_msg_params;
mod test_09_plugin_auth_unpwd_fail;
mod test_09_plugin_auth_unpwd_success;
mod test_09_plugin_auth_v2_unpwd_fail;
mod test_09_plugin_auth_v2_unpwd_success;
mod test_09_plugin_publish;
mod test_09_plugin_tick;
mod test_09_pwfile_parse_invalid;
mod test_10_listener_mount_point;
mod test_11_message_expiry;
mod test_11_persistent_subscription;
mod test_11_persistent_subscription_no_local;
mod test_11_persistent_subscription_v5;
mod test_11_pub_props;
mod test_11_subscription_id;
mod test_12_prop_assigned_client_identifier;
mod test_12_prop_maximum_packet_size_broker;
mod test_12_prop_maximum_packet_size_publish_qos1;
mod test_12_prop_maximum_packet_size_publish_qos2;
mod test_12_prop_response_topic;
mod test_12_prop_response_topic_correlation_data;
mod test_12_prop_server_keepalive;
mod test_12_prop_subpub_content_type;
mod test_12_prop_subpub_payload_format;
mod test_13_malformed_publish_v5;
mod test_13_malformed_subscribe_v5;
mod test_13_malformed_unsubscribe_v5;
mod test_14_dynsec_acl;
mod test_14_dynsec_anon_group;
mod test_14_dynsec_auth;
mod test_14_dynsec_client;
mod test_14_dynsec_client_invalid;
mod test_14_dynsec_default_access;
mod test_14_dynsec_disable_client;
mod test_14_dynsec_group;
mod test_14_dynsec_group_invalid;
mod test_14_dynsec_modify_client;
mod test_14_dynsec_modify_group;
mod test_14_dynsec_modify_role;
mod test_14_dynsec_plugin_invalid;
mod test_14_dynsec_role;
mod test_14_dynsec_role_invalid;