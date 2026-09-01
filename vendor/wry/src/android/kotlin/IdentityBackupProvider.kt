// Copyright 2026 Styrene Labs
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

package {{package}}

import android.content.ContentProvider
import android.content.ContentValues
import android.content.Context
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import java.io.File
import java.io.FileNotFoundException

class IdentityBackupProvider : ContentProvider() {
    override fun onCreate(): Boolean = true

    override fun getType(uri: Uri): String {
        requireShareUri(uri)
        return MIME_TYPE
    }

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        requireShareUri(uri)
        if (mode != "r") {
            throw FileNotFoundException()
        }
        val file = shareFile(requireNotNull(context))
        if (!file.isFile) {
            throw FileNotFoundException()
        }
        return ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
    }

    override fun query(
        uri: Uri,
        projection: Array<out String>?,
        selection: String?,
        selectionArgs: Array<out String>?,
        sortOrder: String?
    ): Cursor {
        requireShareUri(uri)
        val requested = projection ?: arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE)
        val columns = requested.filter {
            it == OpenableColumns.DISPLAY_NAME || it == OpenableColumns.SIZE
        }
        val cursor = MatrixCursor(columns.toTypedArray(), 1)
        val file = shareFile(requireNotNull(context))
        cursor.addRow(columns.map {
            if (it == OpenableColumns.DISPLAY_NAME) FILE_NAME else file.length()
        })
        return cursor
    }

    override fun insert(uri: Uri, values: ContentValues?): Uri? = null

    override fun update(
        uri: Uri,
        values: ContentValues?,
        selection: String?,
        selectionArgs: Array<out String>?
    ): Int = 0

    override fun delete(
        uri: Uri,
        selection: String?,
        selectionArgs: Array<out String>?
    ): Int = 0

    private fun requireShareUri(uri: Uri) {
        val appContext = requireNotNull(context)
        if (uri != shareUri(appContext)) {
            throw FileNotFoundException()
        }
    }

    companion object {
        const val MAX_DOCUMENT_BYTES = 16 * 1024 * 1024
        const val MIME_TYPE = "application/octet-stream"
        private const val DIRECTORY_NAME = "identity-backup-share"
        private const val FILE_NAME = "identity-backup.stid"

        fun shareFile(context: Context): File =
            File(File(context.cacheDir, DIRECTORY_NAME), FILE_NAME)

        fun shareUri(context: Context): Uri = Uri.Builder()
            .scheme("content")
            .authority("${context.packageName}.identity-backup")
            .appendPath(FILE_NAME)
            .build()
    }
}
